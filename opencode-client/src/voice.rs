//! Голос по механике Speech Note (dsnote): каталог моделей (whisper/piper),
//! скачивание с HF, запись с микрофона (parec), транскрипция (whisper-cli),
//! озвучка (piper). Всё локально, пользователь сам выбирает и качает модели.

use crate::cmd::Cmd;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const CATALOG_PATH: &str = "/usr/share/harbour-opencode/models.json";
pub const WHISPER_BIN: &str = "/usr/libexec/harbour-opencode/whisper-cli";
/// Статическая сборка piper (aarch64) ставится в RPM рядом с whisper-cli.
/// Каталог содержит espeak-ng-data и свои .so (RUNPATH=$ORIGIN).
pub const PIPER_HOME: &str = "/usr/libexec/harbour-opencode/piper";
pub const PIPER_BIN: &str = "/usr/libexec/harbour-opencode/piper/piper";
const MODELS_SUBDIR: &str = "harbour-opencode/models";
/// Файл сохранения выбора STT/TTS/языка (переживает перезапуски приложения).
const CONFIG_FILE: &str = "voice_state.json";
const REC_RAW: &str = "/tmp/harbour-opencode-ptt.pcm";
/// Временный WAV для озвучки: синтез (воркер) → файл → QML MediaPlayer.
const TTS_OUT: &str = "/tmp/harbour-opencode-tts.wav";

/// Режим озвучки ответов: [Auto] — озвучивать каждый ответ, [Button] — только
/// по кнопке 🔉 на баббле, [Off] — выключена. Единый источник строк протокола.
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

/// Имена голосовых команд — единый источник: `crate::cmd::Cmd` (voice-варианты).

/// Движки моделей (ключи каталога models.json).
pub mod engine {
    pub const STT_WHISPER: &str = "stt_whisper";
    pub const TTS_PIPER: &str = "tts_piper";
}

/// Фаза скачивания/состояние модели (сериализуется в status JSON для QML).
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

/// Прогресс текущего скачивания + состояния моделей.
#[derive(Default, Clone)]
pub struct DlState {
    pub id: String,
    pub total: u64,
    pub done: u64,
    pub state: Phase,
}

/// Общее состояние голосового модуля (шарено с воркером и QML).
#[derive(Default)]
pub struct VoiceState {
    pub status: Arc<std::sync::Mutex<String>>,
    /// Скачивания по id модели: параллельно может идти сразу несколько.
    pub dl: Arc<std::sync::Mutex<HashMap<String, DlState>>>,
    pub rec: Arc<std::sync::Mutex<Option<tokio::process::Child>>>,
    pub stt_model: Arc<std::sync::Mutex<String>>,
    pub tts_model: Arc<std::sync::Mutex<String>>,
    pub lang: Arc<std::sync::Mutex<String>>,
    /// Режим озвучки ответов (см. [TtsMode]).
    pub tts_mode: Arc<std::sync::Mutex<TtsMode>>,
    /// Модели, качание которых надо отменить (ставит command, проверяет download_model).
    pub cancel_dl: Arc<std::sync::Mutex<HashSet<String>>>,
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

    /// Восстановить выбор STT/TTS/языка из файла (вызывается при старте).
    /// Выбранные модели восстанавливаются только если их файлы реально на диске.
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

    /// Сохранить выбор STT/TTS/языка/режима озвучки в файл (переживает перезапуски).
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

    /// Пересобрать status_json (список моделей + прогресс + выбор).
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
                // Модель «на месте», если на диске присутствуют все файлы каталога
                // (на счётчик байтов не полагаемся: `size` может описывать больше,
                // чем реально качаем — из-за этого был ложный «download failed»).
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
    std::fs::read_to_string(CATALOG_PATH)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "langs": [], "models": [] }))
}

fn base_dir() -> PathBuf {
    data_dir().join("models")
}

/// Директория данных приложения (настройки + модели).
fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/nemo".to_string());
    let data = std::env::var("XDG_DATA_HOME")
        .unwrap_or_else(|_| format!("{home}/.local/share"));
    Path::new(&data).join("harbour-opencode")
}

/// Путь к файлу сохранения голосовых настроек.
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

/// Все имена файлов модели (urls + sups[*].urls): по ним решаем, скачана ли она.
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

/// Модель найдена в каталоге и её файлы реально на диске (валидация выбора).
fn model_ready(id: &str, cat: &Value) -> bool {
    let m = cat["models"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["model_id"].as_str() == Some(id)));
    match m {
        Some(m) => all_present(&model_dir(id), &model_files(m)),
        None => false,
    }
}

/// Скачать модель: все файлы из `urls` (и `sups[*].urls`) в `models/{id}/`.
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
                    // Раз в ~250 мс публикуем прогресс, чтобы QML видел % вживую.
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
            // Отмена: выкидываем частично скачанные файлы и сбрасываем запись.
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
                    q.push(format!("[голос] «{name}» скачана"));
                } else {
                    q.push(format!("[голос] ошибка скачивания «{name}»"));
                }
            }
        }
    }
    voice.refresh_status();
}

/// Запустить транскрипцию wav выбранной STT-моделью.
pub async fn run_stt(voice: &Arc<VoiceState>, pending: &Arc<std::sync::Mutex<Vec<String>>>) {
    let id = voice.stt_model.lock().map(|g| g.clone()).unwrap_or_default();
    if id.is_empty() {
        if let Ok(mut q) = pending.lock() {
            q.push("[голос] сначала выберите STT-модель в настройках".to_string());
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
            q.push("[голос] модель не скачана: скачайте её в настройках".to_string());
        }
        return;
    }
    // PARAM lang
    let lang = voice.lang.lock().map(|g| g.clone()).unwrap_or_default();
    let lang = if lang.is_empty() { "auto".to_string() } else { lang };
    let wav = format!("{REC_RAW}.wav");
    log::info!("STT: модель={id} lang={lang} файл={wav}");
    if let Ok(mut q) = pending.lock() {
        q.push("[голос] распознаю…".to_string());
    }

    let out = match tokio::process::Command::new(WHISPER_BIN)
        .arg("-m").arg(&model_file)
        .arg("-f").arg(&wav)
        .arg("-l").arg(&lang)
        .arg("-t").arg("4")
        .arg("-nt")
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            log::error!("whisper-cli не запустился: {e}");
            if let Ok(mut q) = pending.lock() {
                q.push(format!("[голос] ошибка запуска STT: {e}"));
            }
            return;
        }
    };
    if !out.status.success() {
        log::error!("whisper-cli: {}", String::from_utf8_lossy(&out.stderr));
        if let Ok(mut q) = pending.lock() {
            q.push("[голос] ошибка распознавания (см. лог)".to_string());
        }
        return;
    }
    let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if txt.is_empty() {
        if let Ok(mut q) = pending.lock() {
            q.push("[голос] распознано пусто".to_string());
        }
        return;
    }
    if let Ok(mut q) = pending.lock() {
        q.push(format!("[[stt]] {txt}"));
    }
}

/// 44-байтный RIFF/WAV заголовок для PCM 16 кГц / моно / s16.
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

/// Убить parec и обернуть PCM в WAV.
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

/// STT-точка из QML: спонсит задачу транскрипции.
pub async fn stt_from_call(voice: &Arc<VoiceState>, pending: &Arc<std::sync::Mutex<Vec<String>>>) {
    let voice2 = voice.clone();
    let pending2 = pending.clone();
    tokio::spawn(async move {
        run_stt(&voice2, &pending2).await;
    });
}

/// Найти пути .onnx и .onnx.json выбранной piper-модели в каталоге моделей.
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

/// Синтезировать текст выбранной TTS-моделью в WAV и попросить QML проиграть его.
///
/// Воркер не трогает аудио-устройство: piper пишет `/tmp/harbour-opencode-tts.wav`,
/// воркер пушит строку `[[tts]]<путь>`, QML проигрывает её через MediaPlayer.
/// Так не нужен cpal/Пульс в Rust-процессе, а сама озвучка не блокирует SSE-поток.
pub async fn run_tts(voice: &Arc<VoiceState>, text: &str, pending: &Arc<std::sync::Mutex<Vec<String>>>) {
    let id = voice.tts_model.lock().map(|g| g.clone()).unwrap_or_default();
    if id.is_empty() {
        if let Ok(mut q) = pending.lock() {
            q.push("[голос] сначала выберите TTS-модель в настройках".to_string());
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
            q.push(format!("[голос] модель «{id}» не скачана: скачайте её в настройках"));
        }
        return;
    };
    if !std::path::Path::new(PIPER_BIN).exists() {
        if let Ok(mut q) = pending.lock() {
            q.push(format!("[голос] piper не установлен ({PIPER_BIN})"));
        }
        return;
    }
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    log::info!(
        "TTS: модель={id} piper={PIPER_BIN} текст={} байт",
        text.len()
    );

    // Текст отдаём piper в stdin, поэтому нужен piped stdin + запись.
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;
    use std::process::Stdio;

    let mut child = match Command::new(PIPER_BIN)
        .current_dir(PIPER_HOME) // чтобы espeak-ng-data нашёлся рядом с бинарём
        .arg("-m")
        .arg(&onnx)
        .arg("-c")
        .arg(&config)
        .arg("-f")
        .arg(TTS_OUT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            log::error!("piper не запустился: {e}");
            if let Ok(mut q) = pending.lock() {
                q.push(format!("[голос] ошибка запуска TTS: {e}"));
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
            q.push("[голос] ошибка синтеза (см. лог)".to_string());
        }
        return;
    }
    log::info!("TTS: синтез готов, WAV в {TTS_OUT}");
    if let Ok(mut q) = pending.lock() {
        q.push(format!("[[tts]]{TTS_OUT}"));
    }
}

/// TTS-точка из QML: спонсит задачу озвучки указанного текста.
pub async fn tts_from_call(voice: &Arc<VoiceState>, text: &str, pending: &Arc<std::sync::Mutex<Vec<String>>>) {
    let voice2 = voice.clone();
    let pending2 = pending.clone();
    let text = text.to_string();
    tokio::spawn(async move {
        run_tts(&voice2, &text, &pending2).await;
    });
}

/// Обработать команды голосового модуля (из очереди QML).
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
                // Параллельные скачивания разрешены: каждая модель качается в своём таске.
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
                    q.push(format!("[голос] скачивание «{name}»…"));
                }
                tokio::spawn(async move {
                    download_model(&voice2, &id, &cat, &pending2).await;
                });
            }
        }
        Some(Cmd::VoiceDownloadCancel) => {
            // Отменяем конкретную модель (по id), а не все разом.
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
                // Если удалили выбранную модель — снимаем выбор.
                let cur = voice.stt_model.lock().map(|g| g.clone()).unwrap_or_default();
                let cur2 = voice.tts_model.lock().map(|g| g.clone()).unwrap_or_default();
                if cur == id {
                    if let Ok(mut g) = voice.stt_model.lock() {
                        *g = String::new();
                    }
                }
                if cur2 == id {
                    if let Ok(mut g) = voice.tts_model.lock() {
                        *g = String::new();
                    }
                }
                voice.persist();
                voice.refresh_status();
            }
        }
        Some(Cmd::VoiceRecordStart) => start_recording(voice).await,
        Some(Cmd::VoiceRecordStop) => {
            stop_recording(voice).await;
            // подтверждаем завершение слушания коротким beep-сообщением
            if let Ok(mut q) = pending.lock() {
                q.push("[голос] запись завершена".to_string())
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
        _ => {}
    }
}