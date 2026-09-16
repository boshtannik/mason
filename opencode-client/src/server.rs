use std::io::BufRead;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use log::{debug, error, info};
use tokio::time::sleep;

use crate::api::OpenCodeClient;

/// RAII-владелец процесса `opencode serve`.
///
/// - `start()` — поднимает сервер в конструкторе
/// - `Drop` — останавливает сервер (kill), когда владелец умирает
///
/// Конфликт портов решён: `port = 0` (по умолчанию) — сервер сам
/// выбирает свободный порт, мы читаем его из лога. Другой opencode
/// на телефоне не пересекается.
pub struct ServerConfig {
    /// Порт. 0 = авто-подбор (по умолчанию).
    pub port: u16,
    /// Бинарник opencode ($OPENCODE_BIN → which).
    pub bin: Option<std::path::PathBuf>,
    /// Доп. аргументы (--pure для работы без плагинов).
    pub args: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 0,
            bin: None,
            args: vec!["--pure".to_string()],
        }
    }
}

pub struct ServerGuard {
    pub child: StdMutex<Option<Child>>,
    pub base: String,
    pub port: u16,
    pub auth: Option<(String, String)>,
}

impl ServerGuard {
    /// Запускает `opencode serve`, читает из лога назначенный порт,
    /// ждёт health-ответа и возвращает готовый guard (RAII).
    pub async fn start(cfg: ServerConfig) -> Result<Self, String> {
        let bin = cfg.bin.unwrap_or_else(|| {
            std::ffi::OsString::from(
                std::env::var("OPENCODE_BIN")
                    .unwrap_or_else(|_| "opencode".to_string()),
            )
            .into()
        });

        let mut cmd = Command::new(&bin);
        cmd.arg("serve")
            .arg("--port")
            .arg(cfg.port.to_string())
            .args(&cfg.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!("запускаю: {:?}", cmd);
        let mut child = cmd.spawn().map_err(|e| {
            format!("не удалось запустить `{}`: {e}", bin.display())
        })?;

        // std::sync::mpsc — меж-поточный, без tokio runtime.
        let (tx, rx) = mpsc::channel::<String>();

        if let Some(out) = child.stdout.take() {
            let tx = tx.clone();
            std::thread::spawn(move || read_lines(out, tx));
        }
        if let Some(err) = child.stderr.take() {
            let tx = tx.clone();
            std::thread::spawn(move || read_lines(err, tx));
        }
        drop(tx);

        // Ждём строку "listening on http://127.0.0.1:PORT".
        let mut port: Option<u16> = None;
        const MAX_ATTEMPTS: u32 = 50;
        for _i in 0..MAX_ATTEMPTS {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return Err(format!("opencode serve умер: {status}"));
                }
                Ok(None) => {}
                Err(e) => return Err(format!("try_wait err: {e}")),
            }
            while let Ok(line) = rx.try_recv() {
                debug!("[opencode] {line}");
                if port.is_none() {
                    port = parse_listening_port(&line);
                }
            }
            if port.is_some() {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }

        let port = match port {
            Some(p) => p,
            None => {
                let _ = child.kill();
                return Err("opencode serve не сообщил порт за 5s".into());
            }
        };
        let base = format!("http://127.0.0.1:{port}");
        let auth = None;

        // Фоновый сборщик логов.
        tokio::spawn(async move {
            while let Ok(line) = rx.recv() {
                debug!("[opencode-bg] {line}");
            }
        });

        // Ждём health.
        let client = OpenCodeClient::new(base.clone(), auth.clone());
        let mut ready = false;
        for _ in 0..50 {
            if client.health().await.unwrap_or(false) {
                ready = true;
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
        if !ready {
            let _ = child.kill();
            return Err(format!("opencode serve не ответил на health: {base}"));
        }

        info!("opencode serve готов: port={port}, pid={}", child.id());
        Ok(Self {
            child: StdMutex::new(Some(child)),
            base,
            port,
            auth,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }

    pub fn pid(&self) -> u32 {
        self.child
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| c.id())
            .unwrap_or(0)
    }

    pub async fn health(&self) -> Result<bool, String> {
        let client = OpenCodeClient::new(self.base.clone(), self.auth.clone());
        client.health().await
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.lock().unwrap().take() {
            info!("останавливаю opencode serve (pid={})", child.id());
            match child.kill() {
                Ok(_) => {
                    let _ = child.wait();
                }
                Err(e) => error!("не смог остановить opencode serve: {e}"),
            }
        }
    }
}

fn read_lines<R: std::io::Read>(reader: R, tx: mpsc::Sender<String>) {
    for line in std::io::BufReader::new(reader).lines().flatten() {
        if tx.send(line).is_err() {
            return;
        }
    }
}

/// "[out] opencode server listening on http://127.0.0.1:45067" → Some(45067)
fn parse_listening_port(line: &str) -> Option<u16> {
    let needle = "listening on http://127.0.0.1:";
    let pos = line.find(needle)?;
    let rest = &line[pos + needle.len()..];
    rest.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}