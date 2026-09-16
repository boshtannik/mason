use opencode_client::audio::player::Player;
use opencode_client::audio::recorder::Recorder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().ok();

    // Запись 3 секунд c микрофона через cpal.
    let rec = Recorder::new();
    let wav = rec.record("/tmp/cpal_test.wav", 3)?;
    println!("PASS: записано {} байт", std::fs::metadata(&wav)?.len());
    println!("PASS: файл {}", wav.display());

    // Проигрывание через cpal.
    let player = Player::new();
    player.play(&wav)?;
    println!("PASS: проиграно без ошибок");

    let _ = std::fs::remove_file(&wav);
    Ok(())
}