//! Voice modeled after Speech Note (dsnote): model catalog (whisper/piper),
//! downloading from HF, microphone recording (parec), transcription (whisper-cli),
//! playback (piper). Everything runs locally; the user picks and downloads models.

use crate::cmd::Cmd;
use crate::i18n::{now_ms, svc_line};
use crate::markers::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const CATALOG_PATH: &str = "/usr/share/harbour-opencode/models.json";
/// The fresh model catalog is downloaded from the official dsnote (mkiol) repo —
/// the same format and source the stock RPM catalog was cut from.
pub const CATALOG_URL: &str =
    "https://raw.githubusercontent.com/mkiol/dsnote/main/config/models.json";
/// Engines the app supports (the rest are dropped from the catalog).
pub const SUPPORTED_ENGINES: [&str; 3] = ["stt_whisper", "tts_piper", "tts_espeak"];
const CATALOG_USER_FILE: &str = "catalog.json";
pub const WHISPER_BIN: &str = "/usr/libexec/harbour-opencode/whisper-cli";
/// Static piper build (aarch64) ships in the RPM next to whisper-cli.
/// Its directory holds espeak-ng-data and its own .so files (RUNPATH=$ORIGIN).
pub const PIPER_HOME: &str = "/usr/libexec/harbour-opencode/piper";
pub const PIPER_BIN: &str = "/usr/libexec/harbour-opencode/piper/piper";
const MODELS_SUBDIR: &str = "harbour-opencode/models";
/// File that persists the STT/TTS/language choice (survives app restarts).
const CONFIG_FILE: &str = "voice_state.json";
const REC_RAW: &str = "/tmp/harbour-opencode-ptt.pcm";
/// Temporary WAV for playback: synthesis (worker) → file → QML MediaPlayer.
/// The name is unique per synthesis so several consecutive answers
/// can be played in sequence (queue in QML).
const TTS_OUT_DIR: &str = "/tmp";
static TTS_SEQ: AtomicU64 = AtomicU64::new(0);

fn tts_out_path() -> String {
    format!("{TTS_OUT_DIR}/harbour-opencode-tts-{:03}.wav", TTS_SEQ.fetch_add(1, Ordering::SeqCst))
}

/// Answer playback mode: [Auto] — speak every answer, [Button] — only
/// via the 🔉 button on the bubble, [Off] — off. Single source of protocol strings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TtsMode {
    Auto,
    Button,
    Off,
}

impl Default for TtsMode {
    fn default() -> Self {
        TtsMode::Button
    }
}

impl TtsMode {
    pub const ALL: [TtsMode; 3] = [TtsMode::Auto, TtsMode::Button, TtsMode::Off];

    pub fn as_str(self) -> &'static str {
        match self {
            TtsMode::Auto => "auto",
            TtsMode::Button => "button",
            TtsMode::Off => "off",
        }
    }

    pub fn parse(s: &str) -> Option<TtsMode> {
        match s {
            "auto" => Some(TtsMode::Auto),
            "button" => Some(TtsMode::Button),
            "off" => Some(TtsMode::Off),
            _ => None,
        }
    }
}

/// Voice command names — single source of truth: `crate::cmd::Cmd` (voice variants).

/// Model engines (keys of the models.json catalog).
pub mod engine {
    pub const STT_WHISPER: &str = "stt_whisper";
    pub const TTS_PIPER: &str = "tts_piper";
}

/// Download phase/model state (serialized into status JSON for QML).
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    #[default]
    None,
    Downloading,
    Done,
    Error,
    Ready,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::None => "",
            Phase::Downloading => "downloading",
            Phase::Done => "done",
            Phase::Error => "error",
            Phase::Ready => "ready",
        }
    }
}

/// Progress of the current download + model states.
#[derive(Default, Clone)]
pub struct DlState {
    pub id: String,
    pub total: u64,
    pub done: u64,
    pub state: Phase,
}

/// Shared state of the voice module (shared with the worker and QML).
#[derive(Default)]
pub struct VoiceState {
    pub status: Arc<std::sync::Mutex<String>>,
    /// Downloads keyed by model id: several can run in parallel.
    pub dl: Arc<std::sync::Mutex<HashMap<String, DlState>>>,
    pub rec: Arc<std::sync::Mutex<Option<tokio::process::Child>>>,
    pub stt_model: Arc<std::sync::Mutex<String>>,
    pub tts_model: Arc<std::sync::Mutex<String>>,
    pub lang: Arc<std::sync::Mutex<String>>,
    /// Answer playback mode (see [TtsMode]).
    pub tts_mode: Arc<std::sync::Mutex<TtsMode>>,
    /// Models whose download should be cancelled (set by a command, checked in download_model).
    pub cancel_dl: Arc<std::sync::Mutex<HashSet<String>>>,
    /// Cancel the current synthesis: set to true so run_tts won't push the WAV
    /// into the queue and will delete the file (set by QML stop, checked in run_tts).
    pub cancel_tts: Arc<std::sync::Mutex<bool>>,
    /// Whether a transcription task is running right now. TTS defers while STT runs,
    /// so the two engines don't saturate the CPU at the same time.
    pub stt_busy: Arc<std::sync::Mutex<bool>>,
    /// Whether a synthesis task is running right now. Only one piper at a time:
    /// two parallel answers would race on their output files.
    pub tts_busy: Arc<std::sync::Mutex<bool>>,
}

impl VoiceState {
    pub fn with_status(status: Arc<std::sync::Mutex<String>>) -> Arc<Self> {
        let v = Arc::new(Self {
            status,
            ..Default::default()
        });
        v.load_config();
        v.refresh_status();
        v
    }

    /// Restore the STT/TTS/language choice from the file (called at startup).
    /// Selected models are restored only if their files are actually on disk.
    fn load_config(&self) {
        let Some(path) = config_path() else { return };
        let Ok(raw) = std::fs::read_to_string(&path) else { return };
        let Ok(v) = serde_json::from_str::<Value>(&raw) else {
            log::warn!("не распарсить голосовой конфиг, игнорирую");
            return;
        };
        let cat = load_catalog();

        let stt = v.get("stt").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if !stt.is_empty() {
            if model_ready(&stt, &cat) {
                if let Ok(mut g) = self.stt_model.lock() {
                    *g = stt.clone();
                }
            } else {
                log::info!("STT-модель {stt} не на месте — выбор снят");
            }
        }
        let tts = v.get("tts").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if !tts.is_empty() {
            if model_ready(&tts, &cat) {
                if let Ok(mut g) = self.tts_model.lock() {
                    *g = tts.clone();
                }
            } else {
                log::info!("TTS-модель {tts} не на месте — выбор снят");
            }
        }
        let lang = v.get("lang").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if !lang.is_empty() {
            let known = lang == "auto"
                || cat["langs"]
                    .as_array()
                    .map(|a| a.iter().any(|l| l["id"].as_str() == Some(lang.as_str())))
                    .unwrap_or(false);
            if known {
                if let Ok(mut g) = self.lang.lock() {
                    *g = lang.clone();
                }
            } else {
                log::info!("язык {lang} неизвестен каталогу — сброшен на auto");
            }
        }
        let tts_mode = v
            .get("tts_mode")
            .and_then(|x| x.as_str())
            .and_then(TtsMode::parse)
            .unwrap_or_default();
        if let Ok(mut g) = self.tts_mode.lock() {
            *g = tts_mode;
        }
        log::info!(
            "голосовой конфиг загружен: stt={stt} tts={tts} lang={lang} tts_mode={}",
            tts_mode.as_str()
        );
    }

    /// Save the STT/TTS/language/playback-mode choice to the file (survives restarts).
    pub fn persist(&self) {
        let Some(path) = config_path() else { return };
        let stt = self.stt_model.lock().map(|g| g.clone()).unwrap_or_default();
        let tts = self.tts_model.lock().map(|g| g.clone()).unwrap_or_default();
        let lang = self.lang.lock().map(|g| g.clone()).unwrap_or_default();
        let tts_mode = self
            .tts_mode
            .lock()
            .map(|g| g.as_str().to_string())
            .unwrap_or_else(|_| TtsMode::default().as_str().to_string());
        let json = serde_json::json!({ "stt": stt, "tts": tts, "lang": lang, "tts_mode": tts_mode });
        match std::fs::write(&path, json.to_string()) {
            Ok(()) => log::info!(
                "голосовой конфиг сохранён (stt={stt}, tts={tts}, lang={lang}, tts_mode={tts_mode})"
            ),
            Err(e) => log::error!("не сохранить голосовой конфиг: {e}"),
        }
    }

    pub fn stt_model_handle(&self) -> Arc<std::sync::Mutex<String>> {
        self.stt_model.clone()
    }
    pub fn tts_model_handle(&self) -> Arc<std::sync::Mutex<String>> {
        self.tts_model.clone()
    }
    pub fn lang_handle(&self) -> Arc<std::sync::Mutex<String>> {
        self.lang.clone()
    }
    pub fn tts_mode_handle(&self) -> Arc<std::sync::Mutex<TtsMode>> {
        self.tts_mode.clone()
    }
    pub fn status_handle(&self) -> Arc<std::sync::Mutex<String>> {
        self.status.clone()
    }

    /// Rebuild status_json (model list + progress + selection).
    pub fn refresh_status(&self) {
        let cat = load_catalog();
        let models = &cat["models"];
        let dl = self.dl.lock().map(|g| g.clone()).unwrap_or_default();
        let arr: Vec<Value> = models
            .as_array()
            .map(|v| v.iter().collect::<Vec<_>>())
            .unwrap_or_default()
            .iter()
            .filter_map(|m| {
                let id = m["model_id"].as_str()?;
                let dir = model_dir(id);
                let total: u64 = m["size"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0);
                // A model is "in place" when all catalog files are on disk
                // (we don't trust the byte counter: `size` may describe more than
                // we actually download — that caused a false "download failed").
                let got = all_present(&dir, &model_files(m));
                let mut o = serde_json::Map::new();
                o.insert("model_id".into(), Value::String(id.to_string()));
                o.insert("name".into(), m["name"].clone());
                o.insert("engine".into(), m["engine"].clone());
                o.insert("lang_id".into(), m["lang_id"].clone());
                o.insert("size".into(), m["size"].clone());
                o.insert("downloaded".into(), Value::Bool(got));
                let dlst = dl.get(id);
                let state = if got {
                    Phase::Ready.as_str().to_string()
                } else if let Some(d) = dlst {
                    d.state.as_str().to_string()
                } else {
                    Phase::None.as_str().to_string()
                };
                o.insert("state".into(), Value::String(state));
                if let Some(d) = dlst {
                    o.insert("done".into(), Value::from(d.done));
                    o.insert("total".into(), Value::from(d.total));
                }
                Some(Value::Object(o))
            })
            .collect();
        let langs: Vec<Value> = cat["langs"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|l| {
                        let id = l["id"].as_str()?;
                        let name = l["name_en"]
                            .as_str()
                            .or_else(|| l["name"].as_str())
                            .unwrap_or(id);
                        Some(serde_json::json!({ "id": id, "name": name }))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let json = serde_json::json!({
            "stt": self.stt_model.lock().map(|g| g.clone()).unwrap_or_default(),
            "tts": self.tts_model.lock().map(|g| g.clone()).unwrap_or_default(),
            "lang": self.lang.lock().map(|g| g.clone()).unwrap_or_default(),
            "models": arr,
            "langs": langs,
        });
        if let Ok(mut g) = self.status.lock() {
            *g = json.to_string();
        }
    }
}

pub fn load_catalog() -> Value {
    // First the updated catalog from the user folder (if "Refresh
    // catalogs" was already used), otherwise the stock one from the RPM.
    let user = data_dir().join(CATALOG_USER_FILE);
    let user = (user.exists()).then(|| user);
    user.and_then(|p| std::fs::read_to_string(p).ok())
        .or_else(|| std::fs::read_to_string(CATALOG_PATH).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "langs": [], "models": [] }))
}

/// Download the fresh model catalog from GitHub (mkiol/dsnote) and save it to
/// the user folder. Returns the catalog version and the number of models after
/// filtering by supported engines.
pub async fn update_catalog() -> Result<(u64, usize), String> {
    let client = reqwest::Client::builder()
        .user_agent("harbour-opencode/0.1")
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let body = client
        .get(CATALOG_URL)
        .send()
        .await
        .map_err(|e| format!("скачать каталог: {e}"))?
        .text()
        .await
        .map_err(|e| format!("читать каталог: {e}"))?;
    let mut cat: Value =
        serde_json::from_str(&body).map_err(|e| format!("не JSON каталога: {e}"))?;

    // Keep only the models that the app can download/use.
    if let Some(models) = cat["models"].as_array_mut() {
        models.retain(|m| {
            m["engine"]
                .as_str()
                .map(|e| SUPPORTED_ENGINES.contains(&e))
                .unwrap_or(false)
        });
    }
    let count = cat["models"].as_array().map(|a| a.len()).unwrap_or(0);
    let version: u64 = cat["version"]
        .as_str()
        .and_then(|s| s.trim().parse().ok())
        .or_else(|| cat["version"].as_u64())
        .unwrap_or(0);

    let dir = data_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Err(format!("папka данных: {e}"));
    }
    let (tmp, dest) = (dir.join(format!("{CATALOG_USER_FILE}.tmp")), dir.join(CATALOG_USER_FILE));
    std::fs::write(&tmp, serde_json::to_string_pretty(&cat).unwrap_or(body.clone()))
        .map_err(|e| format!("запись каталога: {e}"))?;
    std::fs::rename(&tmp, &dest).map_err(|e| format!("сохранить каталог: {e}"))?;
    log::info!(
        "каталог моделей обновлён: с GitHub (models={count}) → {}",
        dest.display()
    );
    Ok((version, count))
}

fn base_dir() -> PathBuf {
    data_dir().join("models")
}

/// Application data directory (settings + models).
fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/nemo".to_string());
    let data = std::env::var("XDG_DATA_HOME")
        .unwrap_or_else(|_| format!("{home}/.local/share"));
    Path::new(&data).join("harbour-opencode")
}

/// Path to the voice settings file.
fn config_path() -> Option<PathBuf> {
    std::env::var("HOME").ok()?;
    Some(data_dir().join(CONFIG_FILE))
}

pub fn model_dir(id: &str) -> PathBuf {
    base_dir().join(id)
}

fn model_file(m: &Value) -> Option<PathBuf> {
    let id = m["model_id"].as_str()?;
    let url = m["urls"].as_array()?.first()?.as_str()?;
    let name = url.rsplit('/').next().unwrap_or("model");
    Some(model_dir(id).join(name))
}

/// All model file names (urls + sups[*].urls): by them we decide whether it is downloaded.
fn model_files(m: &Value) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut add = |urls: &Value| {
        if let Some(a) = urls.as_array() {
            for u in a.iter().filter_map(|u| u.as_str()) {
                let n = u.rsplit('/').next().unwrap_or("file").to_string();
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
    };
    add(&m["urls"]);
    if let Some(sups) = m["sups"].as_array() {
        for s in sups {
            add(&s["urls"]);
        }
    }
    names
}

fn all_present(dir: &Path, names: &[String]) -> bool {
    !names.is_empty() && names.iter().all(|n| dir.join(n).exists())
}

/// The model is found in the catalog and its files are actually on disk (selection validation).
fn model_ready(id: &str, cat: &Value) -> bool {
    let m = cat["models"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["model_id"].as_str() == Some(id)));
    match m {
        Some(m) => all_present(&model_dir(id), &model_files(m)),
        None => false,
    }
}

/// Download a model: all files from `urls` (and `sups[*].urls`) into `models/{id}/`.
pub async fn download_model(
    voice: &Arc<VoiceState>,
    id: &str,
    cat: &Value,
    pending: &Arc<std::sync::Mutex<Vec<String>>>,
) {
    let m = cat["models"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["model_id"].as_str() == Some(id)));
    let Some(m) = m else {
        log::warn!("модель {id} не найдена в каталоге");
        return;
    };
    let name = m["name"].as_str().unwrap_or(id);
    let expected: u64 = m["size"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0);
    {
        let mut dl = voice.dl.lock().unwrap();
        dl.insert(
            id.to_string(),
            DlState {
                id: id.to_string(),
                total: expected,
                done: 0,
                state: Phase::Downloading,
            },
        );
    }
    {
        let mut c = voice.cancel_dl.lock().unwrap();
        c.remove(id);
    }
    log::info!("скачивание {id}: старт (размер={expected})");
    voice.refresh_status();

    let mut urls: Vec<String> = m["urls"]
        .as_array()
        .map(|a| a.iter().filter_map(|u| u.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    if let Some(sups) = m["sups"].as_array() {
        for s in sups {
            if let Some(u) = s["urls"].as_array() {
                for uu in u.iter().filter_map(|u| u.as_str()) {
                    urls.push(uu.to_string());
                }
            }
        }
    }
    if urls.is_empty() {
        log::warn!("модель {id}: нет URL");
        return;
    }

    let dir = model_dir(id);
    if let Err(e) = std::fs::create_dir_all(&dir) {
log::error!("models dir: {e}");
            {
                let mut dl = voice.dl.lock().unwrap();
                if let Some(d) = dl.get_mut(id) {
                    d.state = Phase::Error;
                }
            }
        voice.refresh_status();
        return;
    }

    let client = match reqwest::Client::builder()
        .user_agent("harbour-opencode/0.1")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::error!("http client: {e}");
            {
                let mut dl = voice.dl.lock().unwrap();
                if let Some(d) = dl.get_mut(id) {
                    d.state = Phase::Error;
                }
            }
            voice.refresh_status();
            return;
        }
    };

    let mut ok = true;
    for url in urls {
        let name = url.rsplit('/').next().unwrap_or("file").to_string();
        let dest = dir.join(&name);
        if dest.exists() {
            log::info!("{name}: файл уже есть, пропускаю");
            continue;
        }
        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                log::error!("GET {url}: {e}");
                ok = false;
                break;
            }
        };
        log::info!("{name}: скачиваю (content-length={})", resp.content_length().unwrap_or(0));
        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;
        let mut file = match tokio::fs::File::create(&dest).await {
            Ok(f) => f,
            Err(e) => {
                log::error!("create {name}: {e}");
                ok = false;
                break;
            }
        };
        let mut fail = false;
        let mut last_refresh = Instant::now();
        while let Some(chunk) = stream.next().await {
            if let Ok(mut c) = voice.cancel_dl.lock() {
                if c.contains(id) {
                    log::info!("скачивание {id} отменено пользователем");
                    fail = true;
                    break;
                }
            }
            match chunk {
                Ok(c) => {
                    if file.write_all(&c).await.is_err() {
                        fail = true;
                        break;
                    }
                    if let Ok(mut dl) = voice.dl.lock() {
                        if let Some(d) = dl.get_mut(id) {
                            d.done += c.len() as u64;
                        }
                    }
                    // Publish progress every ~250 ms so QML sees % live.
                    if last_refresh.elapsed() >= Duration::from_millis(250) {
                        last_refresh = Instant::now();
                        voice.refresh_status();
                    }
                }
                Err(e) => {
                    log::error!("stream {name}: {e}");
                    fail = true;
                    break;
                }
            }
        }
        let _ = file.flush().await;
        if fail {
            ok = false;
            break;
        }
    }

    {
        let cancelled = voice.cancel_dl.lock().map(|c| c.contains(id)).unwrap_or(true);
        let actual: u64 = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path().metadata().map(|md| md.len()).unwrap_or(0))
                    .sum()
            })
            .unwrap_or(0);
        let mut dl = voice.dl.lock().unwrap();
        if cancelled {
            // Cancelled: throw out the partially downloaded files and reset the record.
            let _ = std::fs::remove_dir_all(&dir);
            dl.remove(id);
        } else if let Some(d) = dl.get_mut(id) {
            d.done = actual;
            let ok_files = all_present(&dir, &model_files(m));
            d.state = if ok && ok_files {
                Phase::Done
            } else {
                Phase::Error
            };
            let state = d.state;
            if let Ok(mut q) = pending.lock() {
                if state == Phase::Done {
                    q.push(svc_line(now_ms(), crate::i18n::K_MODEL_DOWNLOADED, &[name.to_string()]));
                } else {
                    q.push(svc_line(
                        now_ms(),
                        crate::i18n::K_MODEL_DOWNLOAD_FAILED,
                        &[name.to_string()],
                    ));
                }
            }
        }
    }
    voice.refresh_status();
}

/// Run transcription of the wav with the selected STT model.
/// Whisper language code from the system locale (LC_ALL/LC_MESSAGES/LANG).
/// When unset or unrecognised we fall back to `auto` — but note that a
/// real language skips the whisper `-l auto` language-detect pass (a full
/// extra encode of the window), i.e. roughly halves the STT time.
fn stt_default_language() -> String {
    const MAP: &[(&str, &str)] = &[
        ("ru", "ru"), ("be", "be"), ("uk", "uk"), ("en", "en"), ("fi", "fi"), ("sv", "sv"),
        ("de", "de"), ("fr", "fr"), ("es", "es"), ("it", "it"), ("pt", "pt"), ("pl", "pl"),
        ("nl", "nl"), ("cs", "cs"), ("sk", "sk"), ("hu", "hu"), ("ro", "ro"), ("el", "el"),
        ("da", "da"), ("no", "no"), ("et", "et"), ("lv", "lv"), ("lt", "lt"), ("tr", "tr"),
        ("az", "az"), ("kk", "kk"), ("uz", "uz"), ("ka", "ka"), ("hy", "hy"), ("fa", "fa"),
        ("zh", "zh"), ("ar", "ar"), ("hi", "hi"), ("id", "id"), ("vi", "vi"), ("ms", "ms"),
        ("th", "th"), ("ur", "ur"), ("bn", "bn"), ("ta", "ta"), ("te", "te"),
    ];
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(l) = std::env::var(var) {
            let l = l.to_ascii_lowercase();
            let code = l.split(['_', '.', '-', '@']).next().unwrap_or("");
            if let Some((_, w)) = MAP.iter().find(|(c, _)| *c == code) {
                return (*w).to_string();
            }
        }
    }
    "auto".to_string()
}

/// The wrapper marks the transcription as busy; the actual work is in
/// [run_stt_impl] so the flag is cleared on every exit path.
pub async fn run_stt(voice: &Arc<VoiceState>, pending: &Arc<std::sync::Mutex<Vec<String>>>) {
    if let Ok(mut b) = voice.stt_busy.lock() {
        *b = true;
    }
    run_stt_impl(voice, pending).await;
    if let Ok(mut b) = voice.stt_busy.lock() {
        *b = false;
    }
}

async fn run_stt_impl(voice: &Arc<VoiceState>, pending: &Arc<std::sync::Mutex<Vec<String>>>) {
    let id = voice.stt_model.lock().map(|g| g.clone()).unwrap_or_default();
    if id.is_empty() {
        if let Ok(mut q) = pending.lock() {
            q.push(svc_line(now_ms(), crate::i18n::K_STT_NO_MODEL_SELECTED, &[]));
            q.push(proto_line(now_ms(), STT_DONE_TAG));
        }
        return;
    }
    let cat = load_catalog();
    let model = cat["models"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["model_id"].as_str() == Some(id.as_str())));
    let Some(model) = model else {
        return;
    };
    let Some(model_file) = model_file(model) else {
        return;
    };
    if !model_file.exists() {
        if let Ok(mut q) = pending.lock() {
            q.push(svc_line(now_ms(), crate::i18n::K_MODEL_NOT_DOWNLOADED, &[id]));
            q.push(proto_line(now_ms(), STT_DONE_TAG));
        }
        return;
    }
    // PARAM lang. Empty config value = use the app/system language (whisper
    // `auto` runs an extra language-detect encode pass, so prefer a concrete
    // language — the voice page still offers an explicit "auto" option).
    let lang = voice.lang.lock().map(|g| g.clone()).unwrap_or_default();
    let lang = if lang.is_empty() { stt_default_language() } else { lang };
    let wav = format!("{REC_RAW}.wav");
    log::info!("STT: модель={id} lang={lang} файл={wav}");
    // FUTO approach: audio_ctx is trimmed to the actual recording length (frame of 320
    // samples, +32 frames for the decoder), threads by core count, OpenMP from the build.
    let samples = std::fs::metadata(&wav)
        .map(|m| (m.len().saturating_sub(44) / 2) as usize) // samples from 16-bit PCM
        .unwrap_or(0);
    let audio_ctx = (samples / 320 + 32).clamp(272, 1500) as u32;
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(16) as u32;
    log::info!("STT: samples={samples} audio_ctx={audio_ctx} threads={threads}");
    if let Ok(mut q) = pending.lock() {
        q.push(svc_line(now_ms(), crate::i18n::K_STT_RECOGNIZING, &[]));
    }

    // Empty means whisper-cli did not find speech inside the trimmed window. A very short
    // utterance can fall below it, so retry once with the FULL context on an empty result.
    let ctxs: &[u32] = if audio_ctx > 0 { &[audio_ctx, 0] } else { &[0] };
    let mut text: Option<String> = None;
    let mut run_failure: Option<String> = None;
    let mut spawn_failure: Option<String> = None;
    let mut attempts = 0u32;
    for ctx in ctxs.iter().copied() {
        attempts += 1;
        let mut cmd = tokio::process::Command::new(WHISPER_BIN);
        cmd.arg("-m").arg(&model_file)
            .arg("-f").arg(&wav)
            .arg("-l").arg(&lang)
            .arg("-t").arg(threads.to_string())
            .arg("-nt");
        if ctx > 0 {
            cmd.arg("-c").arg(ctx.to_string());
        }
        let out = match cmd.output().await {
            Ok(o) => o,
            Err(e) => {
                spawn_failure = Some(e.to_string());
                break;
            }
        };
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            log::error!("whisper-cli (ctx={ctx}): {stderr}");
            run_failure = Some(stderr);
            break;
        }
        let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !txt.is_empty() {
            text = Some(txt);
            break;
        }
        log::warn!("STT: пустой результат при ctx={ctx}, повторяю с полным окном");
    }

    if let Some(e) = spawn_failure {
        log::error!("whisper-cli не запустился: {e}");
        if let Ok(mut q) = pending.lock() {
            q.push(svc_line(now_ms(), crate::i18n::K_STT_RUN_FAILED, &[e]));
            q.push(proto_line(now_ms(), STT_DONE_TAG));
        }
        return;
    }
    if run_failure.is_some() {
        if let Ok(mut q) = pending.lock() {
            q.push(svc_line(now_ms(), crate::i18n::K_STT_RESULT_FAILED, &[]));
            q.push(proto_line(now_ms(), STT_DONE_TAG));
        }
        return;
    }
    match text {
        Some(txt) => {
            if let Ok(mut q) = pending.lock() {
                q.push(format!("{STT_TAG} {txt}"));
            }
        }
        None => {
            // Diagnostics: how long the recording was when whisper saw nothing.
            let ms = if samples > 0 { samples * 1000 / 16000 } else { 0 };
            log::info!("STT: пусто после {attempts} попыток (запись {ms} мс, {samples} сэмплов)");
            if let Ok(mut q) = pending.lock() {
                q.push(svc_line(now_ms(), crate::i18n::K_STT_EMPTY, &[]));
                q.push(proto_line(now_ms(), STT_DONE_TAG));
            }
        }
    }
}

/// 44-byte RIFF/WAV header for PCM 16 kHz / mono / s16.
fn wav_header(size: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(44);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&(36 + size).to_le_bytes());
    h.extend_from_slice(b"WAVE");
    h.extend_from_slice(b"fmt ");
    h.extend_from_slice(&16u32.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes()); // PCM
    h.extend_from_slice(&1u16.to_le_bytes()); // mono
    h.extend_from_slice(&16000u32.to_le_bytes());
    h.extend_from_slice(&32000u32.to_le_bytes());
    h.extend_from_slice(&2u16.to_le_bytes());
    h.extend_from_slice(&16u16.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&size.to_le_bytes());
    h
}

/// Kill parec and wrap the PCM in a WAV.
pub async fn stop_recording(voice: &Arc<VoiceState>) {
    let child = voice.rec.lock().map(|mut c| c.take()).unwrap_or_default();
    if let Some(mut ch) = child {
        let _ = ch.kill().await;
        let _ = ch.wait().await;
    }
    let raw = Path::new(REC_RAW);
    let wav_str = format!("{REC_RAW}.wav");
    let wav = Path::new(&wav_str);
    match std::fs::read(raw) {
        Ok(pcm) => {
            let mut out = wav_header(pcm.len() as u32);
            out.extend_from_slice(&pcm);
            let _ = std::fs::write(wav, out);
            log::info!("запись остановлена: {} байт → {}", pcm.len(), wav.display());
        }
        Err(e) => log::error!("не прочитан {REC_RAW}: {e}"),
    }
}

pub async fn start_recording(voice: &Arc<VoiceState>) {
    if voice.rec.lock().map(|g| g.is_some()).unwrap_or(false) {
        log::warn!("запись уже идёт");
        return;
    }
    log::info!("запись начата");
    let _ = std::fs::remove_file(REC_RAW);
    let f = match std::fs::File::create(REC_RAW) {
        Ok(f) => f,
        Err(e) => {
            log::error!("не создать {REC_RAW}: {e}");
            return;
        }
    };
    let stdout = std::process::Stdio::from(f);
    let child = tokio::process::Command::new("/usr/bin/parec")
        .args(["--format=s16le", "--rate=16000", "--channels=1"])
        .stdout(stdout)
        .kill_on_drop(true)
        .spawn();
    match child {
        Ok(ch) => {
            if let Ok(mut g) = voice.rec.lock() {
                *g = Some(ch);
            }
        }
        Err(e) => log::error!("parec: {e}"),
    }
}

/// Find the first downloaded model of the engine (for auto-replacement after deletion).
fn replacement_model(exclude_id: &str, engine: &str) -> Option<String> {
    let cat = load_catalog();
    let models = cat["models"].as_array()?;
    for m in models {
        let id = m["model_id"].as_str()?;
        if id == exclude_id {
            continue;
        }
        if m["engine"].as_str() != Some(engine) {
            continue;
        }
        let dir = model_dir(id);
        if !all_present(&dir, &model_files(m)) {
            continue;
        }
        return Some(id.to_string());
    }
    None
}

/// STT entry point from QML: spawns the transcription task.
pub async fn stt_from_call(voice: &Arc<VoiceState>, pending: &Arc<std::sync::Mutex<Vec<String>>>) {
    let voice2 = voice.clone();
    let pending2 = pending.clone();
    tokio::spawn(async move {
        run_stt(&voice2, &pending2).await;
    });
}

/// Find the .onnx and .onnx.json paths of the selected piper model in the model catalog.
fn piper_files(model: &Value) -> Option<(PathBuf, PathBuf)> {
    let id = model["model_id"].as_str()?;
    let dir = model_dir(id);
    let mut onnx: Option<PathBuf> = None;
    let mut json: Option<PathBuf> = None;
    for name in model_files(model) {
        let p = dir.join(&name);
        if name.ends_with(".onnx.json") {
            json = Some(p);
        } else if name.ends_with(".onnx") {
            onnx = Some(p);
        }
    }
    let onnx = onnx?;
    let json = json?;
    (onnx.exists() && json.exists()).then_some((onnx, json))
}

/// Strip markdown formatting before synthesis: headings, lists, block quotes,
/// code fences, inline links and the markup symbols (** ~ # | >) — so the
/// speech engine gets clean prose instead of reading the markup characters.
fn strip_markdown_for_tts(md: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in md.lines() {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || t.is_empty() {
            continue;
        }
        // Horizontal rules.
        if t.chars().all(|c| "-*_~".contains(c)) && t.chars().count() >= 3 {
            continue;
        }
        // Headings, block quotes and table separators: drop the leading marker.
        let rest = t
            .trim_start_matches(|c| c == '#' || c == '>' || c == '|')
            .trim_start();
        // Bullet lists: "- x", "* x", "+ x".
        let rest = rest
            .strip_prefix('-')
            .or_else(|| rest.strip_prefix('*'))
            .or_else(|| rest.strip_prefix('+'))
            .map(|s| s.trim_start())
            .unwrap_or(rest);
        // Numbered lists: "1. x", "12. x".
        let rest = {
            let bytes = rest.as_bytes();
            let mut i = 0;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > 0 && i < bytes.len() && bytes[i] == b'.' {
                rest[i + 1..].trim_start()
            } else {
                rest
            }
        };
        let cleaned = clean_inline_tts(rest);
        if !cleaned.is_empty() {
            out.push_str(&cleaned);
            out.push(' ');
        }
    }
    out
}

/// Inline pass: keep the link label and drop backticks plus the markup symbols.
/// Everything is read character by character, so no regex crate is needed.
fn clean_inline_tts(raw: &str) -> String {
    let mut src = raw;
    let mut out = String::with_capacity(raw.len());
    while !src.is_empty() {
        if let Some(i) = src.find('[') {
            out.push_str(&drop_markup(&src[..i]));
            match src[i..].find(']') {
                Some(j) => {
                    let label = &src[i + 1..i + j];
                    let after = &src[i + j + 1..];
                    // "[label](url)" → label; a lone "[" → label without adornment.
                    if let Some(after) = after.strip_prefix("(") {
                        match after.find(')') {
                            Some(k) => {
                                out.push_str(&clean_inline_tts(label));
                                out.push(' ');
                                src = &after[k + 1..];
                                continue;
                            }
                            None => {}
                        }
                    }
                    out.push_str(&clean_inline_tts(label));
                    out.push(' ');
                    src = after;
                }
                None => {
                    out.push_str(&drop_markup(&src[i..]));
                    break;
                }
            }
        } else {
            out.push_str(&drop_markup(src));
            break;
        }
    }
    out.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Remove the characters that mark bold/italic/strike/code/inline table cells.
fn drop_markup(s: &str) -> String {
    s.chars().filter(|c| !"`*_~|#>".contains(*c)).collect()
}

/// Synthesize the text with the selected TTS model into a WAV and ask QML to play it.
///
/// The worker doesn't touch the audio device: piper writes `/tmp/harbour-opencode-tts.wav`,
/// the worker pushes a `[[tts]]<path>` line, QML plays it via MediaPlayer.
/// This way no cpal/PulseAudio is needed in the Rust process, and the playback itself doesn't block the SSE stream.
pub async fn run_tts(voice: &Arc<VoiceState>, text: &str, pending: &Arc<std::sync::Mutex<Vec<String>>>) {
    let id = voice.tts_model.lock().map(|g| g.clone()).unwrap_or_default();
    if id.is_empty() {
        if let Ok(mut q) = pending.lock() {
            q.push(svc_line(now_ms(), crate::i18n::K_TTS_NO_MODEL_SELECTED, &[]));
        }
        return;
    }
    let cat = load_catalog();
    let model = cat["models"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["model_id"].as_str() == Some(id.as_str())));
    let Some(model) = model else {
        return;
    };
    let Some((onnx, config)) = piper_files(model) else {
        if let Ok(mut q) = pending.lock() {
            q.push(svc_line(now_ms(), crate::i18n::K_MODEL_NOT_DOWNLOADED, &[id]));
        }
        return;
    };
    if !std::path::Path::new(PIPER_BIN).exists() {
        if let Ok(mut q) = pending.lock() {
            q.push(svc_line(
                now_ms(),
                crate::i18n::K_TTS_PIPER_MISSING,
                &[PIPER_BIN.to_string()],
            ));
        }
        return;
    }
    let text = strip_markdown_for_tts(text.trim());
    if text.is_empty() {
        return;
    }
    let out_wav = tts_out_path();
    log::info!(
        "TTS: модель={id} piper={PIPER_BIN} текст={} байт → {out_wav}",
        text.len()
    );

    // We feed the text to piper via stdin, hence piped stdin + write.
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;
    use std::process::Stdio;

    let mut child = match Command::new(PIPER_BIN)
        .current_dir(PIPER_HOME) // so espeak-ng-data is found next to the binary
        .arg("-m")
        .arg(&onnx)
        .arg("-c")
        .arg(&config)
        .arg("-f")
        .arg(&out_wav)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            log::error!("piper не запустился: {e}");
            let emsg = e.to_string();
            if let Ok(mut q) = pending.lock() {
                q.push(svc_line(now_ms(), crate::i18n::K_TTS_RUN_FAILED, &[emsg]));
            }
            return;
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes()).await;
    }
    let out = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => {
            log::error!("piper: {e}");
            return;
        }
    };
    if !out.status.success() {
        log::error!("piper: {}", String::from_utf8_lossy(&out.stderr));
        if let Ok(mut q) = pending.lock() {
            q.push(svc_line(now_ms(), crate::i18n::K_TTS_RESULT_FAILED, &[]));
        }
        return;
    }
    // Synthesis finished, but the user may have managed to press "stop" — then
    // we don't enqueue the result and clean up the file (if it got written).
    if voice.cancel_tts.lock().map(|g| *g).unwrap_or(false) {
        log::info!("TTS: синтез отменён пользователем, файл не отдаю");
        let _ = std::fs::remove_file(&out_wav);
        return;
    }
    log::info!("TTS: синтез готов, WAV в {out_wav}");
    if let Ok(mut q) = pending.lock() {
        q.push(format!("{TTS_TAG}{out_wav}"));
    }
}

/// TTS entry point from QML: spawns the playback task for the given text.
pub async fn tts_from_call(voice: &Arc<VoiceState>, text: &str, pending: &Arc<std::sync::Mutex<Vec<String>>>) {
    let voice2 = voice.clone();
    let pending2 = pending.clone();
    let text = text.to_string();
    // New synthesis = clear the cancel request left by the previous stop.
    if let Ok(mut c) = voice.cancel_tts.lock() {
        *c = false;
    }
    tokio::spawn(async move {
        // Serialize the engines: wait while STT or another synthesis is running (they share
        // the CPU, and two piper jobs would race on their output files). A fresh "stop"
        // press aborts the queued synthesis instead of starting it later.
        loop {
            let busy = {
                let stt = voice2.stt_busy.lock().map(|b| *b).unwrap_or(false);
                let tts = voice2.tts_busy.lock().map(|b| *b).unwrap_or(false);
                stt || tts
            };
            if !busy {
                break;
            }
            if voice2.cancel_tts.lock().map(|c| *c).unwrap_or(false) {
                log::info!("TTS: отменено во время ожидания");
                return;
            }
            tokio::time::sleep(Duration::from_millis(120)).await;
        }
        if let Ok(mut b) = voice2.tts_busy.lock() {
            *b = true;
        }
        run_tts(&voice2, &text, &pending2).await;
        if let Ok(mut b) = voice2.tts_busy.lock() {
            *b = false;
        }
    });
}

/// Handle voice module commands (from the QML queue).
pub async fn run_command(
    voice: &Arc<VoiceState>,
    cmd: &Value,
    pending: &Arc<std::sync::Mutex<Vec<String>>>,
) {
    let action: Option<Cmd> = cmd
        .get("cmd")
        .and_then(|c| serde_json::from_value(c.clone()).ok());
    match action {
        Some(Cmd::VoiceLang) => {
            if let Some(l) = cmd["id"].as_str() {
                if let Ok(mut g) = voice.lang.lock() {
                    *g = l.to_string();
                }
                log::info!("voice lang = {l}");
                voice.persist();
                voice.refresh_status();
            }
        }
        Some(Cmd::VoiceSelectStt) => {
            if let Some(id) = cmd["id"].as_str() {
                if let Ok(mut g) = voice.stt_model.lock() {
                    *g = id.to_string();
                }
                log::info!("STT-модель = {id}");
                voice.persist();
                voice.refresh_status();
            }
        }
        Some(Cmd::VoiceSelectTts) => {
            if let Some(id) = cmd["id"].as_str() {
                if let Ok(mut g) = voice.tts_model.lock() {
                    *g = id.to_string();
                }
                log::info!("TTS-модель = {id}");
                voice.persist();
                voice.refresh_status();
            }
        }
        Some(Cmd::VoiceDownload) => {
            if let Some(id) = cmd["id"].as_str() {
                // Parallel downloads are allowed: each model downloads in its own task.
                if id.is_empty() {
                    return;
                }
                let already = voice
                    .dl
                    .lock()
                    .map(|g| {
                        g.get(id)
                            .map(|d| d.state == Phase::Downloading)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                if already {
                    log::info!("{id}: уже качается");
                    return;
                }
                let cat = load_catalog();
                let voice2 = voice.clone();
                let pending2 = pending.clone();
                let id = id.to_string();
                let name = cat["models"]
                    .as_array()
                    .and_then(|a| a.iter().find(|x| x["model_id"].as_str() == Some(id.as_str())))
                    .and_then(|x| x["name"].as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| id.clone());
                if let Ok(mut q) = pending.lock() {
                    q.push(svc_line(now_ms(), crate::i18n::K_MODEL_DOWNLOADING, &[name]));
                }
                tokio::spawn(async move {
                    download_model(&voice2, &id, &cat, &pending2).await;
                });
            }
        }
        Some(Cmd::VoiceDownloadCancel) => {
            // Cancel the specific model (by id), not all at once.
            if let Some(id) = cmd["id"].as_str() {
                if !id.is_empty() {
                    if let Ok(mut c) = voice.cancel_dl.lock() {
                        c.insert(id.to_string());
                    }
                    log::info!("отмена скачивания {id} запрошена");
                }
            }
        }
        Some(Cmd::VoiceDelete) => {
            if let Some(id) = cmd["id"].as_str() {
                let dir = model_dir(id);
                match std::fs::remove_dir_all(&dir) {
                    Ok(()) => log::info!("модель {id} удалена ({})", dir.display()),
                    Err(e) => log::warn!("не удалилась {id}: {e}"),
                }
                // If the selected model was deleted — clear the selection and pick a replacement
                // (so the user knows which model now does recognition/synthesis).
                let cur = voice.stt_model.lock().map(|g| g.clone()).unwrap_or_default();
                let cur2 = voice.tts_model.lock().map(|g| g.clone()).unwrap_or_default();
                let mut note: Option<String> = None;
                if cur == id {
                    let repl = replacement_model(&cur, "whisper");
                    if let Ok(mut g) = voice.stt_model.lock() {
                        *g = repl.clone().unwrap_or_default();
                    }
                    note = Some(match repl {
                        Some(r) => svc_line(now_ms(), crate::i18n::K_STT_MODEL_SELECTED, &[r]),
                        None => svc_line(now_ms(), crate::i18n::K_STT_MODELS_NONE, &[]),
                    });
                }
                if cur2 == id {
                    let repl = replacement_model(&cur2, "piper");
                    if let Ok(mut g) = voice.tts_model.lock() {
                        *g = repl.clone().unwrap_or_default();
                    }
                    note = match repl {
                        Some(r) => Some(svc_line(now_ms(), crate::i18n::K_TTS_MODEL_SELECTED, &[r])),
                        None => Some(svc_line(now_ms(), crate::i18n::K_TTS_MODELS_NONE, &[])),
                    };
                }
                if let (Some(n), Ok(mut q)) = (note, pending.lock()) {
                    q.push(n);
                }
                voice.persist();
                voice.refresh_status();
            }
        }
        Some(Cmd::VoiceRecordStart) => start_recording(voice).await,
        Some(Cmd::VoiceRecordStop) => {
            stop_recording(voice).await;
            // Confirm the end of listening with a short status message.
            if let Ok(mut q) = pending.lock() {
                q.push(svc_line(now_ms(), crate::i18n::K_STT_RECORD_FINISHED, &[]))
            }
            stt_from_call(voice, pending).await;
        }
        Some(Cmd::VoiceStt) => stt_from_call(voice, pending).await,
        Some(Cmd::VoiceTts) => {
            if let Some(text) = cmd["id"].as_str() {
                if !text.trim().is_empty() {
                    tts_from_call(voice, text, pending).await;
                }
            }
        }
        Some(Cmd::VoiceTtsMode) => {
            if let Some(mode) = cmd["id"].as_str() {
                if let Some(m) = TtsMode::parse(mode) {
                    if let Ok(mut g) = voice.tts_mode.lock() {
                        *g = m;
                    }
                    log::info!("режим озвучки = {}", m.as_str());
                    voice.persist();
                } else {
                    log::warn!("неизвестный режим озвучки: {mode:?}");
                }
            }
        }
        Some(Cmd::VoiceTtsCancel) => {
            // Cancel the current synthesis/playback queue: run_tts checks the flag
            // after piper finishes and won't enqueue the WAV.
            if let Ok(mut c) = voice.cancel_tts.lock() {
                *c = true;
            }
            log::info!("TTS: отмена озвучки запрошена");
        }
        Some(Cmd::VoiceCatalogUpdate) => {
            // Refresh the model catalog from GitHub; report the result as a
            // chat line (like the other worker notifications).
            let pendant = pending.clone();
            let v = voice.clone();
            match update_catalog().await {
                Ok((_version, count)) => {
                    v.refresh_status();
                    let cnt = count.to_string();
                    if let Ok(mut q) = pendant.lock() {
                        q.push(svc_line(
                            now_ms(),
                            crate::i18n::K_CATALOG_REFRESHED,
                            &[cnt],
                        ));
                    }
                }
                Err(e) => {
                    log::error!("обновление каталога: {e}");
                    let emsg = e.to_string();
                    if let Ok(mut q) = pendant.lock() {
                        q.push(svc_line(
                            now_ms(),
                            crate::i18n::K_CATALOG_REFRESH_FAILED,
                            &[emsg],
                        ));
                    }
                }
            }
        }
        // Non-voice commands must not land here: they are handled by
        // the dispatcher in main.rs (its match over Cmd is exhaustive — forgetting
        // a variant when extending is impossible). A catch-all with a log
        // is enough here to notice a stray entry.
        Some(action) => {
            log::warn!("voice::run_command: не-голосовая команда {action:?}");
        }
        None => {
            log::warn!("voice::run_command: отсутствует действие команды");
        }
    }
}