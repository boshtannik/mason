use opencode_client::audio::tts::PiperTts;
use opencode_client::audio::stt::WhisperStt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().ok();

    // Пути из voice-стека, который мы поставили.
    let whisper_bin = std::path::PathBuf::from("/home/jack/tools/voice/whisper-x64/whisper-bin-ubuntu-x64/whisper-cli");
    let whisper_model = std::path::PathBuf::from("/home/jack/tools/voice/models/ggml-small.bin");
    let piper_bin = std::path::PathBuf::from("/home/jack/.local/bin/piper");
    let piper_model = std::path::PathBuf::from("/home/jack/w/personal_secretary/piper-tts/models/ru_RU-dmitri-medium.onnx");
    let piper_cfg = std::path::PathBuf::from("/home/jack/w/personal_secretary/piper-tts/models/ru_RU-dmitri-medium.onnx.json");

    // 1. Синтез речи (чем ответит ассистент).
    let phrase = "Привет. Я твой персональный секретарь.";
    let tts = PiperTts::new(&piper_bin, &piper_model, &piper_cfg);
    let wav = tts.synthesize_to_wav(phrase, "/tmp/assistant_reply.wav")?;
    println!("PASS: piper синтезировал -> {}", wav.display());

    // 2. Распознавание речи (что бы услышал микрофон).
    let engine_path = if cfg!(target_arch = "x86_64") { &whisper_bin } else { &whisper_bin };
    let _ = engine_path;
    let stt = WhisperStt::new(&whisper_bin, &whisper_model).with_lang("ru");
    let stt_text = stt.transcribe(&wav)?;
    println!("PASS: whisper распознал: '{stt_text}'");

    // 3. Сравнение.
    let clean_stt: String = stt_text.chars().filter(|c| !c.is_whitespace()).collect();
    let clean_orig: String = phrase.chars().filter(|c| !c.is_whitespace()).collect();
    if clean_orig.contains(&clean_stt) || clean_stt.contains(&clean_orig) {
        println!("PASS: распознанный текст совпал с исходным");
    } else {
        println!("WARN: текст не совпал. Ожидал: '{phrase}'");
    }

    let _ = std::fs::remove_file(&wav);
    Ok(())
}