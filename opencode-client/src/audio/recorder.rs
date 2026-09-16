use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

/// Запись звука с микрофона через `arecord` (ALSA, есть на Sailfish из коробки).
pub struct Recorder {
    device: String,
    sample_rate: u32,
}

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("arecord не найден — нет ALSA на системе")]
    NotFound,
    #[error("arecord failed: {0}")]
    Io(#[from] std::io::Error),
}

impl Recorder {
    pub fn new() -> Self {
        Self { device: "default".to_string(), sample_rate: 16_000 }
    }

    pub fn with_device(mut self, device: &str) -> Self {
        self.device = device.to_string();
        self
    }

    pub fn with_sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Записать `duration` секунд в WAV-файл.
    ///
    /// Возвращает путь к файлу. FLAC/PCM — `arecord` сам пишет WAV при `.wav` расширении.
    pub fn record(&self, path: impl AsRef<Path>, duration_secs: u32) -> Result<PathBuf, RecordError> {
        let out = path.as_ref().to_path_buf();
        let status = Command::new("arecord")
            .arg("-D")
            .arg(&self.device)
            .arg("-f")
            .arg("S16_LE")
            .arg("-r")
            .arg(self.sample_rate.to_string())
            .arg("-c")
            .arg("1")
            .arg("-t")
            .arg("wav")
            .arg("-d")
            .arg(duration_secs.to_string())
            .arg(&out)
            .status()?;
        if !status.success() {
            return Err(RecordError::NotFound);
        }
        Ok(out)
    }
}