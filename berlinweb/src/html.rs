//! HTML content helpers for the berlinweb UI.
//!
//! Exports static pages (`INDEX_PAGE`, `LOGIN_PAGE`) and the
//! `setup_2fa` helper which renders a QR code and OTP secret. Keep large
//! HTML blobs here to avoid runtime template dependencies.
//!
/// HTML page for the main dashboard with terminal interface and agent list
pub const INDEX_PAGE: &str = r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <title>Rust Agent Hub</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/xterm@5.3.0/css/xterm.css" />
    <script src="https://cdn.jsdelivr.net/npm/xterm@5.3.0/lib/xterm.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/xterm-addon-fit@0.8.0/lib/xterm-addon-fit.js"></script>

    <style>
        :root {
            --bg-dark: #1a1a1a;
            --sidebar-bg: #252526;
            --accent: #007acc;
            --text: #cccccc;
            --active-green: #00ff00;
        }

        body {
            display: flex;
            height: 100vh;
            margin: 0;
            background: var(--bg-dark);
            color: var(--text);
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
        }

        /* Sidebar Styles */
        #sidebar {
            width: 280px;
            background: var(--sidebar-bg);
            border-right: 1px solid #333;
            display: flex;
            flex-direction: column;
        }

        .sidebar-header {
            padding: 15px;
            font-size: 14px;
            font-weight: bold;
            text-transform: uppercase;
            border-bottom: 1px solid #333;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        #agent-list {
            flex-grow: 1;
            overflow-y: auto;
        }

        .agent-item {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 12px 15px;
            cursor: pointer;
            transition: background 0.2s;
            border-bottom: 1px solid #2d2d2d;
        }

        .agent-item:hover {
            background: #37373d;
        }

        .agent-item.active {
            background: #37373d;
            border-left: 3px solid var(--accent);
        }

        .agent-info {
            display: flex;
            align-items: center;
            font-family: 'Cascadia Code', monospace;
            font-size: 12px;
        }

        /* Indicador de Actividad */
        .status-dot {
            width: 8px;
            height: 8px;
            background-color: #444;
            border-radius: 50%;
            margin-right: 10px;
            transition: all 0.3s;
        }

        .status-dot.unread {
            background-color: var(--active-green);
            box-shadow: 0 0 8px var(--active-green);
        }

        .close-btn {
            color: #666;
            font-size: 18px;
            padding: 0 5px;
            line-height: 1;
        }

        .close-btn:hover {
            color: #ff4444;
        }

        /* Terminal Area */
        #terminal-container {
            flex-grow: 1;
            padding: 10px;
            background: var(--bg-dark);
            position: relative;
        }

        #terminal {
            width: 100%;
            height: 100%;
        }

        .placeholder {
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100%;
            color: #555;
            font-style: italic;
        }
        #agent-info-banner {
            background: #2d2d2d;
            color: #eee;
            padding: 10px 20px;
            display: flex;
            gap: 25px;
            font-family: 'Cascadia Code', monospace;
            font-size: 13px;
            border-bottom: 1px solid #444;
            align-items: center;
        }

        .info-label {
            color: var(--accent); 
            font-weight: bold;
            margin-right: 5px;
        }

        .info-value {
            color: #ffffff;
        }
    </style>
</head>
<body>

    <div id="sidebar">
        <div class="sidebar-header">
            <span>Online Agents</span>
            <button onclick="refreshAgents()" style="background:none; border:none; color:var(--accent); cursor:pointer;">🔄</button>
        </div>
        <div id="agent-list">
        </div>
    </div>

 <div id="terminal-container">
    <div id="agent-info-banner" style="display: none;">
        <span id="info-os"></span>
        <span id="info-arch"></span>
        <span id="info-host"></span>
    </div>
    
    <div id="terminal">
        <div class="placeholder">Select an Agent</div>
    </div>
</div>

    <script>
        const sessions = {}; 
        let currentActiveId = null;

        // 1. Refresh List
        async function refreshAgents() {
            try {
                const res = await fetch('/api/agents');
                const ids = await res.json();
                renderAgentList(ids);
            } catch (e) {
                console.error("Error loading agents:", e);
            }
        }

        // 2. Draw List
        function renderAgentList(ids) {
            const list = document.getElementById('agent-list');
            list.innerHTML = ids.map(id => {
                const isActive = id === currentActiveId ? 'active' : '';
                const isUnread = (sessions[id] && sessions[id].unread) ? 'unread' : '';
                
                return `
                    <div class="agent-item ${isActive}" data-id="${id}" onclick="handleAgentClick('${id}')">
                        <div class="agent-info">
                            <div class="status-dot ${isUnread}"></div>
                            <span>${id}</span>
                        </div>
                        <span class="close-btn" onclick="closeSession(event, '${id}')">×</span>
                    </div>
                `;
            }).join('');
        }

        // 3. Click item
        function handleAgentClick(id) {
            if (!sessions[id]) {
                createNewSession(id);
            } else {
                switchToTerminal(id);
            }
        }

        // 4. New Persisten Session
        function createNewSession(id) {
            const term = new Terminal({
                cursorBlink: true,           
                fontFamily: 'monospace',
                theme: {
                    background: '#1a1b26',   
                    foreground: '#cfc9c2',   
                    cursor: '#ff9e64'        
                },
                rows: 35,                    
                cols: 132,                    
            });
            const fitAddon = new FitAddon.FitAddon();
            term.loadAddon(fitAddon);

            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const socket = new WebSocket(`${protocol}//${window.location.host}/ws/${id}`);
            socket.binaryType = 'arraybuffer';

            socket.onmessage = (event) => {
                const data = new Uint8Array(event.data);
                term.write(data);
                
                
                if (currentActiveId !== id) {
                    sessions[id].unread = true;
                    updateAgentUI(id);
                }
            };

            term.onData(data => {
                if (socket.readyState === WebSocket.OPEN) {
                    /*if (data==="\r") {
                         data="\r\n";
                    }*/
                    socket.send(data);
                }
            });

            socket.onclose = () => {
                term.writeln('\r\n\x1b[31m[Agent desconectat]\x1b[0m');
            };

            sessions[id] = { term, socket, unread: false };
            switchToTerminal(id);
        }

        // 5. Change visible terminal
    async function switchToTerminal(id) {
    currentActiveId = id;
    const banner = document.getElementById('agent-info-banner');

    // 1. Cridem a l'API de Rust per obtenir la info de l'agent
    try {
        const response = await fetch(`/api/agent/${id}`);
        if (response.ok) {
            const data = await response.json();
            
            // Triem la icona segons l'OS
            let icon = '💻';
            if (data.os.toLowerCase().includes('win')) icon = '🪟';
            if (data.os.toLowerCase().includes('linux')) icon = '🐧';

            // 2. Pintem la info al banner
            banner.style.display = 'flex';
            banner.innerHTML = `
                <div><span class="info-label">${icon} OS:</span> <span class="info-value">${data.os}</span></div>
                <div><span class="info-label">ARCH:</span> <span class="info-value">${data.arch}</span></div>
                <div><span class="info-label">HOSTNAME:</span> <span class="info-value">${data.hostname}</span></div>
            `;
        }
    } catch (err) {
        console.error("Error obtenint info de l'agent:", err);
        banner.style.display = 'none';
    }

    // 3. Mostrem el terminal com ja ho feies
    const container = document.getElementById('terminal');
    container.innerHTML = ''; 
    sessions[id].term.open(container);
    sessions[id].term.focus();

    // Actualitzem la UI de la llista
    document.querySelectorAll('.agent-item').forEach(el => {
        el.classList.toggle('active', el.dataset.id === id);
    });
}

        // 6. Close Session
async function closeSession(event, id) {
    event.stopPropagation();
    
    // 1. Confirmació (opcional però recomanada)
    if (!confirm(`Vols tancar la sessió de l'agent ${id}?`)) return;

    try {
        // 2. Avisem al servidor perquè tanqui la connexió TCP amb l'agent
        const res = await fetch(`/api/agent/${id}`, { method: 'DELETE' });
        
        if (res.ok) {
            // 3. Si el servidor confirma, netegem la UI local
            if (sessions[id]) {
                sessions[id].socket.close();
                sessions[id].term.dispose();
                delete sessions[id];
            }

            if (currentActiveId === id) {
                document.getElementById('terminal').innerHTML = 
                    '<div class="placeholder">Sessió tancada. Selecciona un altre agent.</div>';
                document.getElementById('agent-info-banner').style.display = 'none';
                currentActiveId = null;
            }
            refreshAgents();
        }
    } catch (e) {
        console.error("Error tancant agent:", e);
    }
}
        // 7. Update Agent UI
        function updateAgentUI(id) {
            const el = document.querySelector(`[data-id="${id}"] .status-dot`);
            if (el) {
                if (sessions[id] && sessions[id].unread) {
                    el.classList.add('unread');
                } else {
                    el.classList.remove('unread');
                }
            }
        }

        // Start
        refreshAgents();
        setInterval(refreshAgents, 5000);
    </script>
</body>
</html>"#;

/// HTML page for authentication with password and OTP input
pub const LOGIN_PAGE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Hub Login</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { background: #121212; color: #e0e0e0; font-family: 'Segoe UI', sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; }
        .login-card { background: #1e1e1e; padding: 2rem; border-radius: 12px; box-shadow: 0 10px 30px rgba(0,0,0,0.5); width: 100%; max-width: 350px; }
        h2 { text-align: center; color: #00ff41; margin-bottom: 1.5rem; font-family: monospace; }
        input { width: 100%; padding: 12px; margin: 10px 0; border-radius: 6px; border: 1px solid #333; background: #252525; color: white; box-sizing: border-box; }
        button { width: 100%; padding: 12px; background: #007bff; border: none; color: white; border-radius: 6px; cursor: pointer; font-weight: bold; margin-top: 10px; }
        button:hover { background: #0056b3; }
        .error { color: #ff4444; font-size: 0.8rem; text-align: center; display: none; }
    </style>
</head>
<body>
    <div class="login-card">
        <h2>HUB_AUTH</h2>
        <form action="/login" method="POST">
            <input type="password" name="password" placeholder="Password" required>
            <input type="text" name="otp_code" placeholder="000000 (OTP)" inputmode="numeric" pattern="[0-9]{6}" required>
            <button type="submit">Sigin</button>
        </form>
        <div id="msg" class="error">Bad credentials</div>
    </div>
    <script>
        if(window.location.search.includes('error')) document.getElementById('msg').style.display='block';
    </script>
</body>
</html>"#;

/// Generate 2FA setup page with QR code and secret key
///
/// # Arguments
/// * `qr_png_b64` - Base64 encoded QR code PNG image
/// * `otp_secret` - Raw OTP secret for manual entry
pub async fn setup_2fa(qr_png_b64: &str, otp_secret: &str) -> String {
    format!(
        "<html><body style='background:#1a1a1a;color:white;text-align:center;padding:50px;'>
            <h2>2FA Configuration</h2>
            <p>Scan Code with Google Authenticator, Microsoft Authenticator, Authy ...:</p>
            <img src='data:image/png;base64,{}' style='border:10px solid white; border-radius:10px;' />
            <p style='margin-top:20px;'>Manual Key: <code>{}</code></p>
            <br><a href='/login' style='color:#007bff;'>Go to Login</a>
        </body></html>",
        qr_png_b64, otp_secret
    )
}
