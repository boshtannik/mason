//! Голос по механике Speech Note (dsnote): каталог моделей (whisper/piper),
//! скачивание с HF, запись с микрофона (parec), транскрипция (whisper-cli),
//! озвучка (piper). Всё локально, пользователь сам выбирает и качает модели.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const CATALOG_PATH: &str = "/usr/share/harbour-opencode/models.json";
pub const WHISPER_BIN: &str = "/usr/libexec/harbour-opencode/whisper-cli";
const MODELS_SUBDIR: &str = "harbour-opencode/models";
const REC_RAW: &str = "/tmp/harbour-opencode-ptt.pcm";

/// Прогресс текущего скачивания + состояния моделей.
#[derive(Default, Clone)]
pub struct DlState {
    pub id: String,
    pub total: u64,
    pub done: u64,
    pub state: String, // "" | "downloading" | "done" | "error"
}

/// Общее состояние голосового модуля (шарено с воркером и QML).
#[derive(Default)]
pub struct VoiceState {
    pub status: Arc<std::sync::Mutex<String>>,
    pub dl: Arc<std::sync::Mutex<DlState>>,
    pub rec: Arc<std::sync::Mutex<Option<tokio::process::Child>>>,
    pub stt_model: Arc<std::sync::Mutex<String>>,
    pub tts_model: Arc<std::sync::Mutex<String>>,
    pub lang: Arc<std::sync::Mutex<String>>,
    /// Флаг отмены текущего скачивания (ставит command, проверяет download_model).
    pub cancel_dl: Arc<std::sync::Mutex<bool>>,
}

impl VoiceState {
    pub fn with_status(status: Arc<std::sync::Mutex<String>>) -> Arc<Self> {
        let v = Arc::new(Self {
            status,
            ..Default::default()
        });
        v.refresh_status();
        v
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
                let got = if dir.is_dir() && total > 0 {
                    let sum: u64 = std::fs::read_dir(&dir)
                        .map(|rd| {
                            rd.flatten()
                                .map(|e| e.path().metadata().map(|md| md.len()).unwrap_or(0))
                                .sum()
                        })
                        .unwrap_or(0);
                    sum >= total
                } else {
                    false
                };
                let mut o = serde_json::Map::new();
                o.insert("model_id".into(), Value::String(id.to_string()));
                o.insert("name".into(), m["name"].clone());
                o.insert("engine".into(), m["engine"].clone());
                o.insert("lang_id".into(), m["lang_id"].clone());
                o.insert("size".into(), m["size"].clone());
                o.insert("downloaded".into(), Value::Bool(got));
                let state = if got {
                    "ready".to_string()
                } else if dl.id == id {
                    dl.state.clone()
                } else {
                    "none".to_string()
                };
                o.insert("state".into(), Value::String(state));
                if dl.id == id {
                    o.insert("done".into(), Value::from(dl.done));
                }
                Some(Value::Object(o))
            })
            .collect();
        let langs: Vec<Value> = cat["langs"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|l| l["id"].as_str().map(|s| Value::String(s.to_string())))
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
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/nemo".to_string());
    let data = std::env::var("XDG_DATA_HOME")
        .unwrap_or_else(|_| format!("{home}/.local/share"));
    Path::new(&data).join(MODELS_SUBDIR)
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

/// Скачать модель: все файлы из `urls` (и `sups[*].urls`) в `models/{id}/`.
pub async fn download_model(voice: &Arc<VoiceState>, id: &str, cat: &Value) {
    let m = cat["models"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["model_id"].as_str() == Some(id)));
    let Some(m) = m else {
        log::warn!("модель {id} не найдена в каталоге");
        return;
    };
    let expected: u64 = m["size"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0);
    {
        let mut dl = voice.dl.lock().unwrap();
        dl.id = id.to_string();
        dl.total = expected;
        dl.done = 0;
        dl.state = "downloading".to_string();
    }
    {
        let mut c = voice.cancel_dl.lock().unwrap();
        *c = false;
    }
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
            dl.state = format!("error: {e}");
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
                dl.state = format!("error: {e}");
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
        while let Some(chunk) = stream.next().await {
            if let Ok(mut c) = voice.cancel_dl.lock() {
                if *c {
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
                        dl.done += c.len() as u64;
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
        let cancelled = voice.cancel_dl.lock().map(|g| *g).unwrap_or(true);
        let actual: u64 = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path().metadata().map(|md| md.len()).unwrap_or(0))
                    .sum()
            })
            .unwrap_or(0);
        if cancelled {
            // Отмена: выкидываем частично скачанные файлы и сбрасываем прогресс.
            let _ = std::fs::remove_dir_all(&dir);
            let mut dl = voice.dl.lock().unwrap();
            dl.id = String::new();
            dl.done = 0;
            dl.state = String::new();
        } else {
            let mut dl = voice.dl.lock().unwrap();
            dl.done = actual;
            dl.state = if ok && (expected == 0 || actual >= expected) {
                "done".to_string()
            } else {
                "error".to_string()
            };
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

/// Обработать команды голосового модуля (из очереди QML).
pub async fn run_command(
    voice: &Arc<VoiceState>,
    cmd: &Value,
    pending: &Arc<std::sync::Mutex<Vec<String>>>,
) {
    match cmd["cmd"].as_str().unwrap_or("") {
        "voice_lang" => {
            if let Some(l) = cmd["id"].as_str() {
                if let Ok(mut g) = voice.lang.lock() {
                    *g = l.to_string();
                }
                log::info!("voice lang = {l}");
                voice.refresh_status();
            }
        }
        "voice_select_stt" => {
            if let Some(id) = cmd["id"].as_str() {
                if let Ok(mut g) = voice.stt_model.lock() {
                    *g = id.to_string();
                }
                log::info!("STT-модель = {id}");
                voice.refresh_status();
            }
        }
        "voice_select_tts" => {
            if let Some(id) = cmd["id"].as_str() {
                if let Ok(mut g) = voice.tts_model.lock() {
                    *g = id.to_string();
                }
                log::info!("TTS-модель = {id}");
                voice.refresh_status();
            }
        }
        "voice_download" => {
            if let Some(id) = cmd["id"].as_str() {
                let busy = voice
                    .dl
                    .lock()
                    .map(|g| g.id == id && g.state == "downloading")
                    .unwrap_or(false);
                if busy {
                    log::info!("{id} уже качается");
                } else {
                    let cat = load_catalog();
                    let voice2 = voice.clone();
                    let id = id.to_string();
                    tokio::spawn(async move {
                        download_model(&voice2, &id, &cat).await;
                    });
                }
            }
        }
        "voice_download_cancel" => {
            if let Ok(mut c) = voice.cancel_dl.lock() {
                *c = true;
            }
            log::info!("отмена скачивания запрошена");
        }
        "voice_delete" => {
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
                voice.refresh_status();
            }
        }
        "voice_record_start" => start_recording(voice).await,
        "voice_record_stop" => {
            stop_recording(voice).await;
            // подтверждаем завершение слушания коротким beep-сообщением
            if let Ok(mut q) = pending.lock() {
                q.push("[голос] запись завершена".to_string())
            }
            stt_from_call(voice, pending).await;
        }
        "voice_stt" => stt_from_call(voice, pending).await,
        _ => {}
    }
}