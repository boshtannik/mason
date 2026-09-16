use std::path::{Path};
use std::process::Command;

use thiserror::Error;

/// Воспроизведение WAV через `aplay` (ALSA, есть на Sailfish).
pub struct Player;

#[derive(Debug, Error)]
pub enum PlayError {
    #[error("aplay не найден — нет ALSA на системе")]
    NotFound,
    #[error("aplay failed: {0}")]
    Io(#[from] std::io::Error),
}

impl Player {
    pub fn new() -> Self {
        Self
    }

    /// Воспроизвести WAV-файл. Возвращает после завершения.
    pub fn play(&self, wav: impl AsRef<Path>) -> Result<(), PlayError> {
        let status = Command::new("aplay")
            .arg("-q")
            .arg(wav.as_ref())
            .status()?;
        if !status.success() {
            return Err(PlayError::NotFound);
        }
        Ok(())
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}