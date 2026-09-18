use std::cell::RefCell;
use std::ffi::CStr;
use std::io::Write as _;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use futures_util::StreamExt;
use qmetaobject::{qml_register_type, QObjectPinned};
use sailors::sailfishapp::QmlApp;
use tokio::sync::Mutex;

use opencode_client::api;
use opencode_client::bridge::app::AppBridge;
use opencode_client::event;
use opencode_client::server;
use opencode_client::voice;

/// Единственный QML-файл приложения (лежит в /usr/share/harbour-opencode/qml).
const MAIN_QML: &str = "qml/harbour-opencode.qml";
/// Имя приложения: определяет и data-каталог `SailfishApp::pathTo`, и argv[0].
const APP_NAME: &str = "harbour-opencode";
/// Сколько ждём готовности opencode serve перед показом окна.
const SERVER_WAIT: Duration = Duration::from_secs(12);

/// Логгер пишет и в stderr, и в файл `~/.cache/harbour-opencode/app.log`
/// (при запуске через booster stdout наследуется и теряется).
struct Tee(std::fs::File);

impl std::io::Write for Tee {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::io::stderr().write_all(buf);
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let _ = std::io::stderr().flush();
        self.0.flush()
    }
}

/// Инициализация логов: `RUST_LOG` (по умолчанию `info`) + файл в кэше приложения.
fn init_logging() {
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    );
    if let Some(home) = std::env::var_os("HOME") {
        let dir = std::path::PathBuf::from(home).join(".cache/harbour-opencode");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("app.log");
        if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            builder.target(env_logger::Target::Pipe(Box::new(Tee(file))));
        }
    }
    let _ = builder.try_init();
    log::info!("--- harbour-opencode starting (pid={}) ---", std::process::id());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    // Мост QML ←→ Rust (создаём до Qt, чтобы отдать очереди воркеру).
    qml_register_type::<AppBridge>(
        CStr::from_bytes_with_nul(b"OpenCodeClient\0")?,
        1,
        0,
        CStr::from_bytes_with_nul(b"AppBridge\0")?,
    );
    let bridge = RefCell::new(AppBridge::default());
    let bridge_pinned = unsafe { QObjectPinned::new(&bridge) };
    let (pending, prompts) = bridge_pinned.borrow().queues();
    let status = bridge_pinned.borrow().status_handle();
    let sessions = bridge_pinned.borrow().sessions_handle();
    let commands = bridge_pinned.borrow().commands_handle();
    let current_session = bridge_pinned.borrow().current_session_handle();
    let nav = bridge_pinned.borrow().nav_handle();
    if let Ok(mut n) = nav.lock() {
        *n = -1;
    }
    let todos = bridge_pinned.borrow().todo_handle();
    let models = bridge_pinned.borrow().models_handle();
    let sounds = bridge_pinned.borrow().sounds_handle();
    let voice = voice::VoiceState::with_status(bridge_pinned.borrow().voice_status_handle());
    bridge_pinned.borrow().set_status_shared("connecting");

    let dispatcher: event::dispatcher::SharedState = Arc::new(Mutex::new(Default::default()));

    // Важно: tokio-runtime нельзя поднимать в главном потоке — при запуске через
    // mapplauncherd/booster (`invoker`, lipstick) он паникует («RefCell already
    // borrowed»). Поэтому весь async-стек живёт в отдельном std-потоке, а Qt —
    // в главном.
    let (url_tx, url_rx) = mpsc::channel::<Option<String>>();
    let env_base = std::env::var("OPENCODE_SERVER_URL").ok();
    let env_auth = std::env::var("OPENCODE_SERVER_PASSWORD").ok().map(|pw| {
        let user =
            std::env::var("OPENCODE_SERVER_USERNAME").unwrap_or_else(|_| "opencode".to_string());
        (user, pw)
    });

    let worker_status = status.clone();
    let worker_sessions = sessions.clone();
    let worker_commands = commands.clone();
    let worker_current = current_session.clone();
    let worker_nav = nav.clone();
    let worker_todos = todos.clone();
    let worker_models = models.clone();
    let worker_sounds = sounds.clone();
    let worker_voice = voice.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(async move {
            let guard = server::ServerGuard::start(server::ServerConfig::default()).await;
            let base = match &guard {
                Ok(g) => {
                    log::info!("opencode serve поднят: {}", g.base_url());
                    Some(env_base.clone().unwrap_or_else(|| g.base_url().to_string()))
                }
                Err(e) => {
                    log::error!("не удалось поднять opencode serve: {e}");
                    env_base.clone()
                }
            };
            let _ = url_tx.send(base.clone());
            match base {
                Some(base) => {
                    run_worker(
                        base,
                        env_auth,
                        dispatcher,
                        pending,
                        prompts,
                        worker_status,
                        worker_sessions,
                        worker_commands,
                        worker_current,
                        worker_nav,
                        worker_todos,
                        worker_models,
                        worker_sounds,
                        worker_voice,
                    )
                    .await
                }
                None => set_status(&worker_status, "error"),
            }
            drop(guard);
        });
    });

    if url_rx.recv_timeout(SERVER_WAIT).ok().flatten().is_none() {
        log::warn!("opencode serve не готов за {:?} — стартуем в offline", SERVER_WAIT);
        set_status(&status, "error");
    }

    // QML в главном потоке; exec() блокирует до закрытия окна.
    let mut app = QmlApp::application(APP_NAME.to_string());
    app.set_object_property("bridge".into(), bridge_pinned);
    app.set_source(QmlApp::path_to(MAIN_QML.into()));
    app.show_full_screen();
    app.exec();

    log::info!("QML закрыт, выходим");
    Ok(())
}

/// Фоновый воркер: (1) SSE-события → QML-мост, (2) промпты из QML → сервер.
#[allow(clippy::too_many_arguments)]
async fn run_worker(
    base: String,
    auth: Option<(String, String)>,
    dispatcher: event::dispatcher::SharedState,
    worker_pending: Arc<std::sync::Mutex<Vec<String>>>,
    worker_prompts: Arc<std::sync::Mutex<Vec<String>>>,
    status: Arc<std::sync::Mutex<String>>,
    sessions: Arc<std::sync::Mutex<String>>,
    commands: Arc<std::sync::Mutex<Vec<String>>>,
    current_session: Arc<std::sync::Mutex<String>>,
    nav: Arc<std::sync::Mutex<i32>>,
    todos: Arc<std::sync::Mutex<String>>,
    models: Arc<std::sync::Mutex<String>>,
    sounds: Arc<std::sync::Mutex<String>>,
    voice: Arc<voice::VoiceState>,
) {
    loop {
        match run_stream(
            base.clone(),
            auth.clone(),
            dispatcher.clone(),
            worker_pending.clone(),
            worker_prompts.clone(),
            status.clone(),
            sessions.clone(),
            commands.clone(),
            current_session.clone(),
            nav.clone(),
            todos.clone(),
            models.clone(),
            sounds.clone(),
            voice.clone(),
        )
        .await
        {
            Ok(()) => log::warn!("SSE поток завершился, переподключаюсь…"),
            Err(e) => log::error!("SSE ошибка: {e}, переподключаюсь…"),
        }
        set_status(&status, "connecting");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Одна SSE-сессия: `Ok(())` — поток закончился, `Err` — не удалось подключиться.
#[allow(clippy::too_many_arguments)]
async fn run_stream(
    base: String,
    auth: Option<(String, String)>,
    dispatcher: event::dispatcher::SharedState,
    worker_pending: Arc<std::sync::Mutex<Vec<String>>>,
    worker_prompts: Arc<std::sync::Mutex<Vec<String>>>,
    status: Arc<std::sync::Mutex<String>>,
    sessions: Arc<std::sync::Mutex<String>>,
    commands: Arc<std::sync::Mutex<Vec<String>>>,
    current_session_shared: Arc<std::sync::Mutex<String>>,
    nav: Arc<std::sync::Mutex<i32>>,
    todos: Arc<std::sync::Mutex<String>>,
    models: Arc<std::sync::Mutex<String>>,
    sounds: Arc<std::sync::Mutex<String>>,
    voice: Arc<voice::VoiceState>,
) -> Result<(), String> {
    let client = api::OpenCodeClient::new(base.clone(), auth.clone());

    let sse = event::sse::SseClient::new(base, auth);
    log::info!("подключаюсь к /global/event...");
    let mut stream = sse.stream().await?;
    log::info!("SSE подключён");
    set_status(&status, "idle");

    // Копим текст ответов по part.id (см. example full_cycle).
    let mut parts: std::collections::HashMap<String, String> = Default::default();
    let mut current_session: Option<String> = None;
    // id ассистентских сообщений — только их текстовые части идят в ответ.
    let mut assistant_messages: std::collections::HashSet<String> = Default::default();
    // Периодический рефреш списка сессий (тикаем каждые 200мс; раз в ~3с — запрос).
    let mut ticks: u32 = 0;

    // Сразу показываем список ранее начатых сессий и доступные модели.
    refresh_sessions(&client, &sessions).await;
    refresh_models(&client, &models).await;
    refresh_sounds(&sounds);

    // Цикл: слушаем SSE и параллельно опрашиваем мост на новые промпты.
    loop {
        tokio::select! {
            item = stream.next() => {
                match item {
                    Some(Ok(ev)) => {
                        if let opencode_client::types::event::Event::SessionError { properties } = &ev.payload {
                            let msg = properties
                                .error
                                .as_ref()
                                .map(|e| e.to_string())
                                .unwrap_or_default();
                            log::error!("session.error: {msg}");
                            if let Ok(mut q) = worker_pending.lock() {
                                q.push(format!("[ошибка сервера] {msg}"));
                            }
                            set_status(&status, "error");
                        }
                        let mut st = dispatcher.lock().await;
                        let changed = event::dispatcher::dispatch(&ev.payload, &mut st).await;
                        drop(st);
                        if let Some(mid) = ev.payload.assistant_message_id() {
                            assistant_messages.insert(mid.to_string());
                        }
                        if let Some((mid, id, text)) = ev.payload.text_part() {
                            if assistant_messages.contains(mid) {
                                parts.insert(id.to_string(), text);
                            }
                        } else if let Some(session_id) = ev.payload.idle_session_id() {
                            if current_session.as_deref() == Some(session_id) {
                                // Собрать накопленный ответ и отдать в QML.
                                let mut ans: Vec<&String> = parts.values().collect();
                                ans.sort();
                                let text: String = ans.iter().map(|s| s.as_str()).collect();
                                if !text.trim().is_empty() {
                                    log::info!("push answer: {:?}", text);
                                    if let Ok(mut q) = worker_pending.lock() {
                                        q.push(text);
                                    }
                                }
                                set_status(&status, "idle");
                                parts.clear();
                                refresh_sessions(&client, &sessions).await;
                            }
                        }
                        let _ = changed;
                    }
                    Some(Err(e)) => log::error!("sse error: {e}"),
                    None => {
                        log::error!("SSE закрылся");
                        set_status(&status, "error");
                        return Ok(());
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                ticks = ticks.wrapping_add(1);

                // Команды из QML: new / open / rename / delete (JSON).
                for raw in commands.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default() {
                    let cmd: serde_json::Value = match serde_json::from_str(&raw) {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("команда не JSON: {e}: {raw}");
                            continue;
                        }
                    };
                    let action = cmd["cmd"].as_str().unwrap_or("");
                    let sid = cmd["id"].as_str().unwrap_or("");
                    match action {
                        "new" => match client.create_session().await {
                            Ok(sess) => match sess["id"].as_str() {
                                Some(id) => {
                                    log::info!("new session: {id}");
                                    current_session = Some(id.to_string());
                                    parts.clear();
                                    assistant_messages.clear();
                                    refresh_sessions(&client, &sessions).await;
                                    set_current_session(&current_session_shared, &current_session);
                                    if let Ok(mut g) = todos.lock() {
                                        *g = "[]".to_string();
                                    }
                                }
                                None => log::error!("new: в ответе нет session.id: {sess}"),
                            },
                            Err(e) => log::error!("new_session: {e}"),
                        },
                        "open" if !sid.is_empty() => {
                            log::info!("open session: {sid}");
                            current_session = Some(sid.to_string());
                            parts.clear();
                            assistant_messages.clear();
                            load_history(&client, sid, &worker_pending).await;
                            set_status(&status, "idle");
                            set_current_session(&current_session_shared, &current_session);
                            refresh_todo(&client, sid, &todos).await;
                        }
                        "fork" if !sid.is_empty() => {
                            match client.fork_session(sid).await {
                                Ok(sess) => match sess["id"].as_str() {
                                    Some(new_id) => {
                                        log::info!("fork {sid} -> {new_id}");
                                        current_session = Some(new_id.to_string());
                                        parts.clear();
                                        assistant_messages.clear();
                                        load_history(&client, new_id, &worker_pending).await;
                                        refresh_sessions(&client, &sessions).await;
                                        set_current_session(&current_session_shared, &current_session);
                                        refresh_todo(&client, new_id, &todos).await;
                                        request_nav(&nav, 2);
                                    }
                                    None => log::error!("fork: в ответе нет id: {sess}"),
                                },
                                Err(e) => log::error!("fork_session: {e}"),
                            }
                        }
                        "share" if !sid.is_empty() => {
                            match client.share_session(sid).await {
                                Ok(sess) => {
                                    let url = sess["share"]["url"].as_str().unwrap_or("");
                                    log::info!("share {sid} -> {url}");
                                    refresh_sessions(&client, &sessions).await;
                                }
                                Err(e) => log::error!("share_session: {e}"),
                            }
                        }
                        "unshare" if !sid.is_empty() => {
                            match client.unshare_session(sid).await {
                                Ok(()) => {
                                    log::info!("unshare {sid}");
                                    refresh_sessions(&client, &sessions).await;
                                }
                                Err(e) => log::error!("unshare_session: {e}"),
                            }
                        }
                        "summarize" if !sid.is_empty() => {
                            match client.get_session(sid).await {
                                Ok(s) => {
                                    let provider = s["model"]["providerID"].as_str().unwrap_or("");
                                    let model = s["model"]["id"].as_str().unwrap_or("");
                                    if provider.is_empty() || model.is_empty() {
                                        log::error!("summarize: у сессии нет модели: {s}");
                                    } else {
                                        match client.summarize_session(sid, provider, model).await {
                                            Ok(()) => log::info!("summarize {sid} ({provider}/{model})"),
                                            Err(e) => log::error!("summarize_session: {e}"),
                                        }
                                    }
                                }
                                Err(e) => log::error!("get_session: {e}"),
                            }
                        }
                        "abort" if !sid.is_empty() => {
                            match client.abort(sid).await {
                                Ok(_) => {
                                    log::info!("abort {sid}");
                                    set_status(&status, "idle");
                                }
                                Err(e) => log::error!("abort: {e}"),
                            }
                        }
                        "set_model" if !sid.is_empty() => {
                            let provider = cmd["provider"].as_str().unwrap_or("");
                            let model = cmd["model"].as_str().unwrap_or("");
                            match client.set_model(sid, provider, model).await {
                                Ok(()) => {
                                    log::info!("set_model {sid} -> {provider}/{model}");
                                    refresh_sessions(&client, &sessions).await;
                                }
                                Err(e) => log::error!("set_model: {e}"),
                            }
                        }
                        "rename" if !sid.is_empty() => {
                            let title = cmd["title"].as_str().unwrap_or("");
                            match client.rename_session(sid, title).await {
                                Ok(()) => {
                                    log::info!("renamed {sid} -> {title:?}");
                                    refresh_sessions(&client, &sessions).await;
                                }
                                Err(e) => log::error!("rename_session: {e}"),
                            }
                        }
                        "delete" if !sid.is_empty() => {
                            match client.delete_session(sid).await {
                                Ok(()) => {
                                    log::info!("deleted {sid}");
                                    if current_session.as_deref() == Some(sid) {
                                        current_session = None;
                                        set_current_session(&current_session_shared, &current_session);
                                        if let Ok(mut g) = todos.lock() {
                                            *g = "[]".to_string();
                                        }
                                    }
                                    refresh_sessions(&client, &sessions).await;
                                }
                                Err(e) => log::error!("delete_session: {e}"),
                            }
                        }
                        "voice_lang"
                        | "voice_select_stt"
                        | "voice_select_tts"
                        | "voice_download"
                        | "voice_record_start"
                        | "voice_record_stop"
                        | "voice_stt" => {
                            voice::run_command(&voice, &cmd, &worker_pending).await;
                        }
                        other => log::warn!("неизвестная команда: {other} ({raw})"),
                    }
                }

                for prompt in worker_prompts.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default() {
                    log::info!("QML промпт: {prompt}");
                    set_status(&status, "busy");
                    // Создаём сессию если ещё нет.
                    if current_session.is_none() {
                        match client.create_session().await {
                            Ok(sess) => match sess["id"].as_str() {
                                Some(id) => {
                                    current_session = Some(id.to_string());
                                    set_current_session(&current_session_shared, &current_session);
                                }
                                None => {
                                    let msg = format!("в ответе нет session.id: {sess}");
                                    log::error!("{msg}");
                                    if let Ok(mut q) = worker_pending.lock() {
                                        q.push(format!("[ошибка сессии] {msg}"));
                                    }
                                    set_status(&status, "error");
                                    continue;
                                }
                            },
                            Err(e) => {
                                log::error!("create_session: {e}");
                                if let Ok(mut q) = worker_pending.lock() {
                                    q.push(format!("[ошибка сессии] {e}"));
                                }
                                set_status(&status, "error");
                                continue;
                            }
                        }
                    }
                    if let Some(sid) = current_session.clone() {
                        if let Err(e) = client.prompt_async(&sid, &prompt).await {
                            log::error!("prompt_async: {e}");
                            if let Ok(mut q) = worker_pending.lock() {
                                q.push(format!("[ошибка промпта] {e}"));
                            }
                            set_status(&status, "error");
                        }
                    }
                }

                // Раз в ~3 секунды обновляем список сессий и TODO текущей сессии.
                if ticks % 15 == 0 {
                    refresh_sessions(&client, &sessions).await;
                    if let Some(sid) = current_session.clone() {
                        refresh_todo(&client, &sid, &todos).await;
                    }
                }
            }
        }
    }
}

/// Загрузить компактный список сессий в общий буфер для QML.
async fn refresh_sessions(
    client: &api::OpenCodeClient,
    sessions: &Arc<std::sync::Mutex<String>>,
) {
    match client.list_sessions().await {
        Ok(v) => {
            let compact: Vec<serde_json::Value> = v
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|s| {
                            let id = s.get("id")?.clone();
                            let title = s
                                .get("title")
                                .cloned()
                                .unwrap_or(serde_json::Value::String(String::new()));
                            let updated = s
                                .get("time")
                                .and_then(|t| t.get("updated"))
                                .cloned()
                                .unwrap_or(serde_json::Value::Number(0.into()));
                            let share_url = s
                                .get("share")
                                .and_then(|sh| sh.get("url"))
                                .cloned()
                                .unwrap_or(serde_json::Value::String(String::new()));
                            let provider_id = s
                                .get("model")
                                .and_then(|m| m.get("providerID"))
                                .cloned()
                                .unwrap_or(serde_json::Value::String(String::new()));
                            let model_id = s
                                .get("model")
                                .and_then(|m| m.get("id"))
                                .cloned()
                                .unwrap_or(serde_json::Value::String(String::new()));
                            let cost = s.get("cost").cloned().unwrap_or(serde_json::Value::Number(0.into()));
                            let tokens = s.get("tokens").cloned().unwrap_or(serde_json::Value::Null);
                            Some(serde_json::json!({
                                "id": id,
                                "title": title,
                                "updated": updated,
                                "shareUrl": share_url,
                                "providerID": provider_id,
                                "modelID": model_id,
                                "cost": cost,
                                "tokens": tokens,
                            }))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let json = serde_json::Value::Array(compact).to_string();
            if let Ok(mut g) = sessions.lock() {
                *g = json;
            }
        }
        Err(e) => log::error!("list_sessions: {e}"),
    }
}

/// Загрузить TODO-задачи сессии в общий буфер для QML.
async fn refresh_todo(
    client: &api::OpenCodeClient,
    session_id: &str,
    todos: &Arc<std::sync::Mutex<String>>,
) {
    match client.session_todo(session_id).await {
        Ok(v) => {
            if let Ok(mut g) = todos.lock() {
                *g = v.to_string();
            }
        }
        Err(e) => log::error!("session_todo: {e}"),
    }
}

/// Загрузить провайдеров/модели и разложить плоско для QML.
async fn refresh_models(
    client: &api::OpenCodeClient,
    models: &Arc<std::sync::Mutex<String>>,
) {
    let v = match client.providers().await {
        Ok(v) => v,
        Err(e) => {
            log::error!("providers: {e}");
            return;
        }
    };
    let mut flat: Vec<serde_json::Value> = Vec::new();
    if let Some(providers) = v["providers"].as_array() {
        for p in providers {
            let pid = p["id"].as_str().unwrap_or("");
            let pname = p["name"].as_str().unwrap_or(pid);
            if let Some(models_map) = p["models"].as_object() {
                let mut entries: Vec<(&String, &serde_json::Value)> = models_map.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                for (mid, m) in entries {
                    let mname = m["name"].as_str().unwrap_or(mid);
                    flat.push(serde_json::json!({
                        "providerID": pid,
                        "providerName": pname,
                        "modelID": mid,
                        "modelName": mname,
                    }));
                }
            }
        }
    }
    let json = serde_json::Value::Array(flat).to_string();
    if let Ok(mut g) = models.lock() {
        *g = json;
    }
}

/// Просканировать системные директории звуков и собрать JSON `[{name,path}]`
/// для QML-выбора (SoundEffect играет только WAV).
fn refresh_sounds(sounds: &Arc<std::sync::Mutex<String>>) {
    let dirs = [
        "/usr/share/sounds/jolla-ambient/stereo",
        "/usr/share/sounds/jolla-ringtones/stereo",
        "/usr/share/sounds/freedesktop/stereo",
    ];
    let mut items: Vec<(String, String)> = Vec::new();
    for dir in dirs {
        let mut in_dir: Vec<(String, String)> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                if !e.path().is_file() {
                    continue;
                }
                let p = e.path();
                let ext = p
                    .extension()
                    .map(|x| x.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                if ext != "wav" {
                    continue;
                }
                let disp = p.display().to_string();
                let fname = p
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let pretty = fname
                    .trim_end_matches(".wav")
                    .replace('_', " ")
                    .replace('-', " ");
                let pretty = pretty
                    .chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == 0 { c.to_uppercase().collect::<String>() } else { c.to_string() }
                    })
                    .collect::<String>();
                in_dir.push((pretty, format!("file://{disp}")));
            }
        }
        in_dir.sort_by(|a, b| a.0.cmp(&b.0));
        items.extend(in_dir);
    }
    let arr: Vec<serde_json::Value> = items
        .iter()
        .map(|(n, p)| serde_json::json!({ "name": n, "path": p }))
        .collect();
    log::info!("найдено системных звуков: {}", arr.len());
    if let Ok(mut g) = sounds.lock() {
        *g = serde_json::Value::Array(arr).to_string();
    }
}

/// Загрузить историю сессии (пары info/parts) и положить строки в очередь QML.
async fn load_history(
    client: &api::OpenCodeClient,
    session_id: &str,
    pending: &Arc<std::sync::Mutex<Vec<String>>>,
) {
    match client.session_messages(session_id).await {
        Ok(v) => {
            let mut items: Vec<&serde_json::Value> =
                v.as_array().map(|a| a.iter().collect()).unwrap_or_default();
            items.sort_by_key(|e| e["info"]["time"]["created"].as_u64().unwrap_or(0));
            let mut lines: Vec<String> = Vec::new();
            for e in items {
                let role = e["info"]["role"].as_str().unwrap_or("");
                let mut text = String::new();
                if let Some(parts) = e["parts"].as_array() {
                    for p in parts {
                        if p["type"].as_str() == Some("text") {
                            if let Some(t) = p["text"].as_str() {
                                text.push_str(t);
                            }
                        }
                    }
                }
                if text.trim().is_empty() {
                    continue;
                }
                if role == "user" {
                    lines.push(format!(">>> {text}"));
                } else {
                    lines.push(text);
                }
            }
            log::info!("история сессии {session_id}: {} строк", lines.len());
            if let Ok(mut q) = pending.lock() {
                q.extend(lines);
            }
        }
        Err(e) => log::error!("session_messages: {e}"),
    }
}

/// Потокобезопасно выставить статус для QML (без Qt-сигналов).
fn set_status(status: &Arc<std::sync::Mutex<String>>, s: &str) {
    if let Ok(mut g) = status.lock() {
        *g = s.to_string();
    }
}

/// Опубликовать id текущей сессии для QML (`None` -> пустая строка).
fn set_current_session(shared: &Arc<std::sync::Mutex<String>>, session: &Option<String>) {
    if let Ok(mut g) = shared.lock() {
        *g = session.clone().unwrap_or_default();
    }
}

/// Попросить QML переключить страницу карусели (0..3) при следующем polling.
fn request_nav(nav: &Arc<std::sync::Mutex<i32>>, page: i32) {
    if let Ok(mut n) = nav.lock() {
        *n = page;
    }
}
