use serde::{Deserialize, Serialize};

/// Information sent by an agent during the initial handshake.
///
/// This struct is serialized and transmitted from the agent to the hub to
/// identify the connecting machine. Fields convey the agent's operating
/// system, CPU architecture, and hostname which are displayed in the web UI.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentInfo {
    /// Operating system string (e.g., "windows", "linux", "macos").
    pub os: String,
    /// CPU architecture (e.g., "x86_64", "aarch64").
    pub arch: String,
    /// Hostname of the agent machine.
    pub hostname: String,
}