//! Web server module for BerlinRC.
//!
//! Provides the HTTPS UI, authentication, WebSocket endpoints for browser
//! clients, and a TCP listener for agents. Manages `ShellSession`s which
//! bridge data between connected agents and browser WebSocket clients.
//!
use axum::{
    Form, Json, Router,
    extract::{
        Path, Request, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    middleware::Next,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use axum_server::tls_rustls::RustlsConfig;
use base64::{Engine as _, engine::general_purpose};
use berlinproto::{handshake::AgentInfo, otp::MyOtp, xor::XorCipher};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{RwLock, mpsc},
};

use crate::{
    config::CONFIG,
    html::{INDEX_PAGE, LOGIN_PAGE, setup_2fa},
};

/// Represents a terminal session for an agent
pub(crate) struct ShellSession {
    // Sender for input data to the agent
    pub(crate) stdin_tx: mpsc::UnboundedSender<Vec<u8>>,
    // Sender for output data to WebSocket clients
    pub(crate) stdout_ws_tx: RwLock<Option<mpsc::UnboundedSender<Vec<u8>>>>,
    // AgentInfo
    pub(crate) ageninfo: Option<AgentInfo>,
    // History
    pub(crate) output_history: RwLock<Vec<u8>>,
    // CancelationToken
    pub(crate) cancel_token: CancellationToken
}

/// Application state containing all active shell sessions
pub(crate) struct AppState {
    /// Map of agent IDs to their respective shell sessions
    pub(crate) sessions: RwLock<HashMap<String, Arc<ShellSession>>>,
}

/// Start the web server with TCP agent listener and HTTPS support
pub async fn run() {
    let state = Arc::new(AppState {
        sessions: RwLock::new(HashMap::new()),
    });

    let tcp_state = Arc::clone(&state);
    tokio::spawn(async move {
        let addr = format!("0.0.0.0:{}", CONFIG.hub_port);
        let listener = TcpListener::bind(addr).await.unwrap();
        println!(
            "🚀 Hub TCP listening for agents on port {}",
            CONFIG.hub_port
        );
        loop {
            let (socket, addr) = listener.accept().await.unwrap();
            let id = addr.to_string();
            let id = id.replace(".", "_").replace(":", "_");
            tokio::spawn(handle_tcp_agent(socket, id, Arc::clone(&tcp_state)));
        }
    });

    let config = RustlsConfig::from_pem(
        CONFIG.cert.as_bytes().to_vec(),
        CONFIG.key.as_bytes().to_vec(),
    )
    .await
    .unwrap();

    let app = Router::new()
        .route("/", get(index_page))
        .route("/api/agents", get(list_agents))
        .route("/api/agent/{id}", get(info_agent).delete(kill_agent))
        .route("/ws/{id}", get(ws_handler))
        .with_state(state)
        .layer(axum::middleware::from_fn(auth_middleware))
        .route("/login", post(login_handler).get(show_login_page))
        .route("/setup-2fa", get(setup_2fa_handler));

    println!("🌐 Web Server UI at https://localhost/ws/[ID]");

    let addr = format!("0.0.0.0:{}", CONFIG.web_port)
        .parse::<std::net::SocketAddr>()
        .unwrap();

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

/// Get list of connected agent IDs
async fn list_agents(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let sessions = state.sessions.read().await;
    Json(sessions.keys().cloned().collect())
}

pub async fn info_agent(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<AgentInfo> {
    let sessions = state.sessions.read().await;
    let agent = sessions.get(&id).expect("Agent not found");
    Json(agent.ageninfo.clone().unwrap())
}
pub(crate) async fn kill_agent(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    let mut sessions = state.sessions.write().await;
    
    if let Some(session) = sessions.remove(&id) {
        println!("💀 Killing agent session: {}", id);
        session.cancel_token.cancel();
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}
/// WebSocket handler for browser client to agent communication
async fn ws_handler(
    jar: CookieJar,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    let auth = jar
        .get("authenticated")
        .map(|c| c.value() == "true")
        .unwrap_or(false);

    if !auth {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    ws.on_upgrade(move |socket| handle_ws(socket, id, state))
}

/// Handle incoming TCP agent connection with XOR encryption
async fn handle_tcp_agent(mut socket: TcpStream, id: String, state: Arc<AppState>) {
    let mut buffer = [0; 512];
    let n = socket.read(&mut buffer).await.unwrap();
    let mut decryptor = XorCipher::new(); 
    decryptor.apply(&mut buffer);
    drop(decryptor);
    let info: AgentInfo = serde_json::from_slice(&buffer[..n]).unwrap();

    println!("🚀 New Agent Connected!");
    println!("   💻 OS: {} ({})", info.os, info.arch);
    println!("   🏠 Hostname: {}", info.hostname);

    let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let mut decryptor_from_agent = XorCipher::new();
    let mut encryptor_to_agent = XorCipher::new();
    let token = CancellationToken::new();
    let session = Arc::new(ShellSession {
        stdin_tx,
        stdout_ws_tx: RwLock::new(None),
        ageninfo: Some(info),
        output_history: RwLock::new(Vec::new()),
        cancel_token: token.clone(),
    });

    let ids = id.clone();

    state
        .sessions
        .write()
        .await
        .insert(id, Arc::clone(&session));

    //println!("✅ Connected Agent: {}", ids);
    let _ = socket.write_all(&[1]).await; // Enviem el "ACK"

    let (mut reader, mut writer) = socket.split();
    let mut buf = [0; 4096];

    loop {
        tokio::select! {
        _ = token.cancelled() => {
            println!("🛑 Cancellation token received. Force closing TCP... {}", ids);
            break; 
        }
        res = reader.read(&mut buf) => {
                match res {
                    Ok(0) => {
                        println!("❌ Agent 0     {}", ids);
                        break; // Conexión cerrada
                    }
                    Ok(n) => {
                        let mut data = buf[..n].to_vec();
                        decryptor_from_agent.apply(&mut data);
                        // --- 🟢 AQUÍ POSEM EL HISTÒRIC ---
                        {
                            let mut history = session.output_history.write().await;
                            history.extend_from_slice(&data);

                            // Opcional: Limitar el buffer a 10KB perquè no creixi infinitament
                            let len=history.len();
                            if len > 10000 {
                                history.drain(0..len - 10000);
                            }
                        }
                        // ---------------------------------



                        let ws_guard = session.stdout_ws_tx.read().await;
                        if let Some(ws_tx) = &*ws_guard {
                            if ws_tx.send(data).is_err() {
                                drop(ws_guard);
                                *session.stdout_ws_tx.write().await = None;
                            }
                        }
                    }
                    Err(_) => {
                        println!("❌ Agent Err   : {}", ids);
                        break; // Error de lectura
                    }
                   
                }
            }
            Some(data) = stdin_rx.recv() => {
                let mut data=data;
                encryptor_to_agent.apply(&mut data);

                if writer.write_all(&data).await.is_err() {
                    break;
                }
            }
        }
    }

    state.sessions.write().await.remove(&ids);
    println!("❌ Agent Disconnected and session Terminated {}", ids);
}

/// Bridge communication between WebSocket and TCP agent
async fn handle_ws(socket: WebSocket, id: String, state: Arc<AppState>) {
    let session = {
        let s = state.sessions.read().await;
        s.get(&id).cloned()
    };

    let session = match session {
        Some(s) => s,
        None => return,
    };

    let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    {
        let mut ws_tx_guard = session.stdout_ws_tx.write().await;
        *ws_tx_guard = Some(ws_tx);
    }

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // --- 🟢 AQUÍ ENVIEM EL HISTÒRIC AL NAVEGADOR ---
    {
        let history = session.output_history.read().await;
        if !history.is_empty() {
            // Enviem tot el que teníem guardat (el banner de PowerShell, etc.)
            if ws_sender
                .send(Message::Binary(history.clone().into()))
                .await
                .is_err()
            {
                return; // Si falla, sortim
            }
        }
    }
    // ----------------------------------------------

    let session_input = Arc::clone(&session);
    let mut task_web_to_agent = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if session_input
                .stdin_tx
                .send(msg.into_data().to_vec())
                .is_err()
            {
                break;
            }
        }
    });

    let mut task_agent_to_web = tokio::spawn(async move {
        while let Some(data) = ws_rx.recv().await {
            if ws_sender.send(Message::Binary(data.into())).await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut task_web_to_agent) => task_agent_to_web.abort(),
        _ = (&mut task_agent_to_web) => task_web_to_agent.abort(),
    }

    if let Some(s) = state.sessions.read().await.get(&id) {
        let mut ws_tx_guard = s.stdout_ws_tx.write().await;
        *ws_tx_guard = None;
    }
}

/// Form data for login authentication
#[derive(Deserialize)]
struct LoginRequest {
    /// User password
    pub password: String,
    /// One-Time Password code
    pub otp_code: String,
}

/// Handle user login with password and OTP verification
async fn login_handler(jar: CookieJar, Form(payload): Form<LoginRequest>) -> impl IntoResponse {
    let otp_secret = CONFIG.otp_secret.as_str();
    let otp = MyOtp::new(otp_secret);

    if payload.password == CONFIG.password && otp.verify(&payload.otp_code) {
        let cookie = Cookie::build(("authenticated", "true"))
            .path("/")
            .http_only(true)
            .same_site(axum_extra::extract::cookie::SameSite::Lax);

        (jar.add(cookie), Redirect::to("/"))
    } else {
        (jar, Redirect::to("/login?error=1"))
    }
}

/// Middleware to enforce authentication on protected routes
async fn auth_middleware(
    jar: CookieJar,
    req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let path = req.uri().path();

    if path == "/login" || path == "/static/login.css" {
        return Ok(next.run(req).await);
    }

    let is_authenticated = jar
        .get("authenticated")
        .map(|c| c.value() == "true")
        .unwrap_or(false);

    if is_authenticated {
        Ok(next.run(req).await)
    } else {
        Err(Redirect::to("/login"))
    }
}

/// Generate and display 2FA setup page with QR code
async fn setup_2fa_handler() -> impl IntoResponse {
    let otp_secret = CONFIG.otp_secret.as_str();
    let otp = MyOtp::new(otp_secret);
    let qr_png = otp.get_qr_png();
    let qr_png_b64 = general_purpose::STANDARD.encode(&qr_png.unwrap());

    Html(setup_2fa(qr_png_b64.as_str(), otp_secret).await)
}

/// Display login page
async fn show_login_page() -> Html<&'static str> {
    Html(LOGIN_PAGE)
}

/// Display main dashboard page
async fn index_page() -> Html<&'static str> {
    Html(INDEX_PAGE)
}
