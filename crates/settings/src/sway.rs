//! Native Sway i3-IPC client for querying and configuring display outputs.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 6] = b"i3-ipc";
const RUN_COMMAND: u32 = 0;
const GET_OUTPUTS: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub name: String,
    #[serde(default)]
    pub make: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_active")]
    pub active: bool,
    #[serde(default = "default_scale")]
    pub scale: f64,
    pub current_mode: Option<Mode>,
    #[serde(default)]
    pub modes: Vec<Mode>,
}

fn default_active() -> bool {
    true
}

fn default_scale() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mode {
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub refresh: i32,
}

impl Mode {
    pub fn refresh_hz(&self) -> f64 {
        self.refresh as f64 / 1000.0
    }
}

fn connect() -> Result<UnixStream, String> {
    let path = std::env::var_os("SWAYSOCK")
        .ok_or_else(|| "SWAYSOCK environment variable is not set (is Sway running?)".to_string())?;
    UnixStream::connect(path).map_err(|e| format!("Cannot connect to Sway IPC: {e}"))
}

fn send(stream: &mut UnixStream, message_type: u32, payload: &[u8]) -> std::io::Result<()> {
    let mut header = [0u8; 14];
    header[..6].copy_from_slice(MAGIC);
    header[6..10].copy_from_slice(&(payload.len() as u32).to_ne_bytes());
    header[10..14].copy_from_slice(&message_type.to_ne_bytes());
    stream.write_all(&header)?;
    stream.write_all(payload)
}

fn recv(stream: &mut UnixStream) -> std::io::Result<(u32, Vec<u8>)> {
    let mut header = [0u8; 14];
    stream.read_exact(&mut header)?;
    if &header[..6] != MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid i3-IPC frame header",
        ));
    }
    let length = u32::from_ne_bytes(header[6..10].try_into().unwrap()) as usize;
    let message_type = u32::from_ne_bytes(header[10..14].try_into().unwrap());
    let mut payload = vec![0u8; length];
    stream.read_exact(&mut payload)?;
    Ok((message_type, payload))
}

/// Query connected display outputs from Sway.
pub fn get_outputs() -> Result<Vec<Output>, String> {
    let mut stream = connect()?;
    send(&mut stream, GET_OUTPUTS, b"").map_err(|e| format!("Failed to send query: {e}"))?;
    let (_, payload) = recv(&mut stream).map_err(|e| format!("Failed to read reply: {e}"))?;
    serde_json::from_slice::<Vec<Output>>(&payload)
        .map_err(|e| format!("Failed to parse Sway output JSON: {e}"))
}

#[derive(Deserialize)]
struct CommandResult {
    success: bool,
    error: Option<String>,
}

/// Apply a new resolution and scale to a display output in Sway.
pub fn set_mode(output: &str, width: i32, height: i32, scale: f64) -> Result<(), String> {
    let mut stream = connect()?;
    let command = format!("output \"{output}\" mode {width}x{height} scale {scale}");
    send(&mut stream, RUN_COMMAND, command.as_bytes())
        .map_err(|e| format!("Failed to send command: {e}"))?;
    let (_, payload) = recv(&mut stream).map_err(|e| format!("Failed to read reply: {e}"))?;
    let results: Vec<CommandResult> = serde_json::from_slice(&payload)
        .map_err(|e| format!("Failed to parse Sway command response: {e}"))?;
    if let Some(err) = results.iter().find(|r| !r.success) {
        return Err(err.error.clone().unwrap_or_else(|| "Command failed in Sway".to_string()));
    }
    Ok(())
}
