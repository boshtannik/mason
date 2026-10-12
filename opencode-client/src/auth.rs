//! Provider credentials that the user can enter in-app.
//!
//! The OpenCode provider ("big-pickle" models) is authenticated with a plain
//! API key obtained from <https://opencode.ai/auth>. The key is stored on the
//! server side via `PUT /auth/{provider}`; the server writes it to
//! `~/.local/share/opencode/auth.json` in the form
//! `{"<provider>": {"type": "api", "key": "..."}}`.

/// Provider id of the OpenCode (Zen) provider.
pub const OPENCODE_PROVIDER: &str = "opencode";

/// Browser page where the user creates / copies their OpenCode API key.
pub const OPENCODE_AUTH_URL: &str = "https://opencode.ai/auth";

/// Path to opencode's credential store (`$XDG_DATA_HOME/opencode/auth.json`,
/// defaulting to `~/.local/share/opencode/auth.json`).
pub fn auth_json_path() -> Option<std::path::PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))?;
    Some(data_home.join("opencode").join("auth.json"))
}

/// Whether an API-key credential is stored for `provider`.
/// Whether an API-key credential is stored for `provider`.
pub fn has_api_key(provider: &str) -> bool {
    let Some(path) = auth_json_path() else {
        return false;
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    json.get(provider)
        .and_then(|v| v.get("key"))
        .and_then(|k| k.as_str())
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false)
}

// ── OpenCode Console device login ─────────────────────────────────────────────
//
// The `opencode` provider's account method is an OAuth 2.0 device authorization
// grant (RFC 8628) against `https://opencode.ai/console`, exposed by the CLI as
// `opencode console login`. The CLI prints
//
//     Go to:  https://opencode.ai/console/device?user_code=XXXX-XXXX&client_id=…
//     Enter code: XXXX-XXXX
//
// and then polls until the user authorizes (or the code expires). We do NOT
// reimplement the flow: writing the credential into the CLI's own store is
// version-specific, so we spawn the real CLI, parse the two lines out of its
// (ANSI) output, show them in the UI, and let the CLI persist the result.

/// Visual state of a device login, serialized to QML as JSON.
#[derive(Clone, Default, serde::Serialize)]
pub struct LoginStatus {
    /// "idle" | "waiting" | "success" | "error"
    pub state: String,
    /// Verification URL the user must open.
    pub url: String,
    /// The user code to enter there.
    pub code: String,
    /// Human-readable detail (failure reason), empty otherwise.
    pub message: String,
}

/// Owns the `opencode console login` child and its shared state.
///
/// Cheap to clone (the state is behind `Arc`); one instance is shared between
/// the QML bridge (reads `status_json`) and the worker (starts/cancels it).
#[derive(Clone, Default)]
pub struct ConsoleLogin {
    status: std::sync::Arc<std::sync::Mutex<LoginStatus>>,
    child: std::sync::Arc<std::sync::Mutex<Option<std::process::Child>>>,
    /// Set to `Some(success)` when the child exits; taken once by the worker.
    finished: std::sync::Arc<std::sync::Mutex<Option<bool>>>,
}

impl ConsoleLogin {
    /// Current state as JSON (`LoginStatus`), read by QML on every poll.
    pub fn status_json(&self) -> String {
        let st = self.status.lock().map(|g| g.clone()).unwrap_or_default();
        serde_json::to_string(&st).unwrap_or_else(|_| "{\"state\":\"error\"}".to_string())
    }

    fn set_state(&self, state: &str, message: &str) {
        if let Ok(mut g) = self.status.lock() {
            g.state = state.to_string();
            g.message = message.to_string();
        }
    }

    fn set_device(&self, url: Option<&str>, code: Option<&str>) {
        if let Ok(mut g) = self.status.lock() {
            if let Some(u) = url {
                g.url = u.to_string();
            }
            if let Some(c) = code {
                g.code = c.to_string();
            }
        }
    }

    /// Start the login child. `Err` if one is already running.
    pub fn start(&self) -> Result<(), String> {
        {
            let g = self.child.lock().map_err(|_| "login lock poisoned".to_string())?;
            if g.is_some() {
                return Err("login already in progress".to_string());
            }
        }

        let bin = crate::server::resolve_bin(None);
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/defaultuser".to_string());
        let mut cmd = std::process::Command::new(&bin);
        cmd.arg("console")
            .arg("login")
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", format!("{home}/.config"))
            .env("XDG_DATA_HOME", format!("{home}/.local/share"))
            .env("XDG_CACHE_HOME", format!("{home}/.cache"))
            .env("XDG_STATE_HOME", format!("{home}/.local/state"))
            .env(
                "PATH",
                std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".to_string()),
            )
            // The TUI would otherwise try to launch a browser; the app opens the
            // verification URL itself via Qt.openUrlExternally.
            .env("BROWSER", "/bin/true")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to start `{}`: {e}", bin.display()))?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        if let Ok(mut g) = self.child.lock() {
            *g = Some(child);
        }
        if let Ok(mut g) = self.status.lock() {
            *g = LoginStatus { state: "waiting".to_string(), ..Default::default() };
        }

        if let Some(out) = stdout {
            let this = self.clone();
            std::thread::spawn(move || this.read_stream(out));
        }
        if let Some(err) = stderr {
            let this = self.clone();
            std::thread::spawn(move || this.read_stream(err));
        }
        let this = self.clone();
        std::thread::spawn(move || this.wait_for_exit());
        Ok(())
    }

    /// Kill the child (user pressed Cancel). Resets to "idle".
    pub fn cancel(&self) {
        let child = self.child.lock().ok().and_then(|mut g| g.take());
        if let Some(mut child) = child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Ok(mut g) = self.status.lock() {
            if g.state == "waiting" {
                *g = LoginStatus::default();
            }
        }
    }

    /// Take the finish flag once (`Some(true)` = success, `Some(false)` = error).
    pub fn take_finished(&self) -> Option<bool> {
        self.finished
            .lock()
            .map(|mut f| f.take())
            .unwrap_or(None)
    }

    /// Last detail message (for the failure notice).
    pub fn message(&self) -> String {
        self.status.lock().map(|g| g.message.clone()).unwrap_or_default()
    }

    fn read_stream<R: std::io::Read>(&self, r: R) {
        use std::io::BufRead;
        for line in std::io::BufReader::new(r).lines().map_while(Result::ok) {
            let clean = strip_ansi(&line);
            if let Some(i) = clean.find("Go to:") {
                let url = clean[i + "Go to:".len()..].trim().split_whitespace().next().unwrap_or("");
                if !url.is_empty() {
                    self.set_device(Some(url), None);
                }
            }
            if let Some(i) = clean.find("Enter code:") {
                let code =
                    clean[i + "Enter code:".len()..].trim().split_whitespace().next().unwrap_or("");
                if !code.is_empty() {
                    self.set_device(None, Some(code));
                }
            }
        }
    }

    fn wait_for_exit(&self) {
        loop {
            let outcome: Option<Result<bool, String>> = {
                let mut g = match self.child.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                match g.as_mut() {
                    Some(child) => match child.try_wait() {
                        Ok(Some(status)) => {
                            *g = None;
                            Some(Ok(status.success()))
                        }
                        Ok(None) => None,
                        Err(e) => {
                            *g = None;
                            Some(Err(e.to_string()))
                        }
                    },
                    None => return, // cancelled
                }
            };
            match outcome {
                Some(Ok(true)) => {
                    self.set_state("success", "");
                    if let Ok(mut f) = self.finished.lock() {
                        *f = Some(true);
                    }
                    return;
                }
                Some(Ok(false)) => {
                    self.set_state("error", "authorization failed or expired");
                    if let Ok(mut f) = self.finished.lock() {
                        *f = Some(false);
                    }
                    return;
                }
                Some(Err(e)) => {
                    self.set_state("error", &e);
                    if let Ok(mut f) = self.finished.lock() {
                        *f = Some(false);
                    }
                    return;
                }
                None => std::thread::sleep(std::time::Duration::from_millis(200)),
            }
        }
    }
}

/// Remove ECMA-48 CSI escape sequences (`ESC [ … <final>`), carriage returns
/// and other control bytes the TUI sprinkles over its output.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\u{1b}' {
            if it.peek() == Some(&'[') {
                it.next();
                while let Some(n) = it.next() {
                    if ('@'..='~').contains(&n) {
                        break;
                    }
                }
            }
            continue;
        }
        if c == '\r' || c == '\u{0}' {
            continue;
        }
        out.push(c);
    }
    out
}

