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
        // Сначала прибираем наши сироты от прошлых аварийных запусков.
        reap_orphans();

        let bin = resolve_bin(cfg.bin);
        info!("PATH={:?}, opencode bin={:?}", std::env::var("PATH"), bin);

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
        // Запоминаем свой PID: только эти процессы мы вправе останавливать.
        let pid = child.id();
        register_spawn(pid);

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
                    unregister_spawn(pid);
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
                unregister_spawn(pid);
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
            unregister_spawn(pid);
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
            let pid = child.id();
            info!("останавливаю opencode serve (pid={pid})");
            match child.kill() {
                Ok(_) => {
                    let _ = child.wait();
                }
                Err(e) => error!("не смог остановить opencode serve: {e}"),
            }
            unregister_spawn(pid);
        }
    }
}

// --- Учёт поднятых нами `opencode serve` ----------------------------------
//
// mapplauncherd-booster иногда убивает приложение так, что `Drop` не
// выполняется, и `opencode serve` остаётся висеть сиротой. Чтобы при
// следующем старте прибрать ИМЕННО СВОИ процессы (и никогда не тронуть
// `opencode`, запущенный пользователем из терминала), ведём реестр:
//   pid + время старта из /proc/<pid>/stat (защита от переиспользования PID).
// Файл: $XDG_DATA_HOME/harbour-opencode/spawned-servers.tsv

extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

const SIGTERM: i32 = 15;
const SIGKILL: i32 = 9;

fn registry_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".local/share"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    base.join("harbour-opencode").join("spawned-servers.tsv")
}

/// Поле 22 `starttime` из /proc/<pid>/stat (в тиках). `None`, если процесса нет.
fn proc_start_time(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // comm заключён в скобки и может содержать ')' — режем по ПОСЛЕДНЕЙ ')'.
    let after = stat.rsplit_once(')')?.1;
    // После comm поле state — это 3-е поле, значит starttime (22) — 20-е здесь.
    after.split_whitespace().nth(19)?.parse().ok()
}

fn proc_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// Действительно ли по PID сейчас `opencode ... serve` (доп. страховка).
fn is_opencode_serve(pid: u32) -> bool {
    match std::fs::read(format!("/proc/{pid}/cmdline")) {
        Ok(bytes) => {
            let s = String::from_utf8_lossy(&bytes).replace('\0', " ");
            s.contains("opencode") && s.contains("serve")
        }
        Err(_) => false,
    }
}

fn kill_pid(pid: u32) {
    for sig in [SIGTERM, SIGKILL] {
        unsafe { kill(pid as i32, sig) };
        for _ in 0..10 {
            if !proc_alive(pid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

fn read_registry() -> Vec<(u32, u64)> {
    std::fs::read_to_string(registry_path())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
        })
        .collect()
}

fn write_registry(records: &[(u32, u64)]) {
    let path = registry_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body: String = records.iter().map(|(p, t)| format!("{p} {t}\n")).collect();
    let _ = std::fs::write(path, body);
}

fn register_spawn(pid: u32) {
    if let Some(start) = proc_start_time(pid) {
        let mut rec = read_registry();
        rec.retain(|(p, _)| *p != pid);
        rec.push((pid, start));
        write_registry(&rec);
    }
}

fn unregister_spawn(pid: u32) {
    let mut rec = read_registry();
    let before = rec.len();
    rec.retain(|(p, _)| *p != pid);
    if rec.len() != before {
        write_registry(&rec);
    }
}

/// Останавливает только НАШИ осиротевшие серверы из реестра.
fn reap_orphans() {
    let records = read_registry();
    if records.is_empty() {
        return;
    }
    let mut alive = Vec::new();
    for (pid, start) in records {
        // Мёртвый или переиспользованный PID — просто забываем.
        if proc_start_time(pid) != Some(start) {
            continue;
        }
        if is_opencode_serve(pid) {
            info!("подчищаю осиротевший opencode serve (pid={pid})");
            kill_pid(pid);
        } else {
            debug!("pid={pid} из реестра уже не opencode serve — не трогаю");
        }
        if proc_alive(pid) {
            alive.push((pid, start));
        }
    }
    write_registry(&alive);
}

/// Ищет бинарь opencode: явный путь → OPENCODE_BIN → стандартные места → PATH.
fn resolve_bin(explicit: Option<std::path::PathBuf>) -> std::path::PathBuf {
    if let Some(b) = explicit {
        return b;
    }
    if let Ok(p) = std::env::var("OPENCODE_BIN") {
        return p.into();
    }
    for cand in [
        "/usr/local/bin/opencode",
        "/usr/bin/opencode",
        "/opt/opencode/bin/opencode",
    ] {
        if std::path::Path::new(cand).exists() {
            return cand.into();
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() {
        let p = format!("{home}/.opencode/bin/opencode");
        if std::path::Path::new(&p).exists() {
            return p.into();
        }
    }
    "opencode".into()
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