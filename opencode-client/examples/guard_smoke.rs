use std::process::Command;
use std::time::Duration;

use opencode_client::server::{ServerConfig, ServerGuard};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::try_init().ok();

    let pid;
    {
        let guard = ServerGuard::start(ServerConfig::default()).await?;
        pid = guard.pid();
        println!("PASS: сервер поднялся, pid={}, base={}", pid, guard.base_url());
        println!("PASS: port={}", guard.port);
        println!("PASS: health -> {}", guard.health().await?);
        tokio::time::sleep(Duration::from_secs(1)).await;
        println!("...сервер жив, падает guard...");
    } // <- тут Drop убивает процесс

    println!("PASS: guard упал, проверяю что pid={} мёртв", pid);
    let alive = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()?
        .success();
    if alive {
        println!("FAIL: процесс {pid} всё ещё жив!");
        std::process::exit(1);
    }
    println!("PASS: процесс мёртв. RAII работает.");
    Ok(())
}