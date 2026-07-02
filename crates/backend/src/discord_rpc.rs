use std::io::{Read, Write};
use std::path::Path;
use std::sync::OnceLock;

use parking_lot::Mutex;
use serde_json::json;

struct DiscordRpcState {
    stream: Option<Box<dyn ReadWrite + Send>>,
    application_id: Option<String>,
    connected: bool,
}

trait ReadWrite: Read + Write {}

impl ReadWrite for std::fs::File {}

#[cfg(unix)]
impl ReadWrite for std::os::unix::net::UnixStream {}

static STATE: OnceLock<Mutex<DiscordRpcState>> = OnceLock::new();

const DEFAULT_APPLICATION_ID: &str = "1522327079381631067";

pub fn init(launcher_dir: &Path) {
    let application_id = resolve_application_id(launcher_dir);
    let _ = STATE.set(Mutex::new(DiscordRpcState {
        stream: None,
        application_id,
        connected: false,
    }));
}

fn resolve_application_id(launcher_dir: &Path) -> Option<String> {
    if let Ok(id) = std::env::var("QUARTZ_DISCORD_APPLICATION_ID") {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return Some(id);
        }
    }

    let file_path = launcher_dir.join("discord_application_id");
    if let Ok(contents) = std::fs::read_to_string(&file_path) {
        let id = contents.trim().to_string();
        if !id.is_empty() {
            return Some(id);
        }
    }

    Some(DEFAULT_APPLICATION_ID.to_string())
}

fn connect_stream() -> Option<Box<dyn ReadWrite + Send>> {
    for index in 0..10 {
        #[cfg(windows)]
        {
            let pipe = format!(r"\\.\pipe\discord-ipc-{index}");
            if let Ok(file) = std::fs::OpenOptions::new().read(true).write(true).open(&pipe) {
                return Some(Box::new(file));
            }
        }

        #[cfg(unix)]
        {
            let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                .or_else(|_| std::env::var("TMPDIR"))
                .unwrap_or_else(|_| "/tmp".to_string());
            let pipe = format!("{runtime_dir}/discord-ipc-{index}");
            if let Ok(stream) = std::os::unix::net::UnixStream::connect(&pipe) {
                return Some(Box::new(stream));
            }

            let tmp_pipe = format!("/tmp/discord-ipc-{index}");
            if let Ok(stream) = std::os::unix::net::UnixStream::connect(&tmp_pipe) {
                return Some(Box::new(stream));
            }
        }
    }

    None
}

fn write_frame(stream: &mut dyn ReadWrite, opcode: u32, payload: &serde_json::Value) -> std::io::Result<()> {
    let data = serde_json::to_vec(payload)?;
    stream.write_all(&opcode.to_le_bytes())?;
    stream.write_all(&(data.len() as u32).to_le_bytes())?;
    stream.write_all(&data)?;
    stream.flush()
}

fn ensure_connected(state: &mut DiscordRpcState) -> bool {
    if state.connected {
        return true;
    }

    let Some(application_id) = state.application_id.as_ref() else {
        return false;
    };

    let Some(mut stream) = connect_stream() else {
        log::debug!("Discord Rich Presence unavailable: Discord IPC pipe not found");
        return false;
    };

    let handshake = json!({
        "v": 1,
        "client_id": application_id,
    });

    if let Err(err) = write_frame(stream.as_mut(), 0, &handshake) {
        log::debug!("Discord Rich Presence handshake failed: {err}");
        return false;
    }

    state.stream = Some(stream);
    state.connected = true;
    true
}

fn set_activity(state: &mut DiscordRpcState, details: &str, state_text: &str) {
    if !ensure_connected(state) {
        return;
    }

    let Some(stream) = state.stream.as_mut() else {
        return;
    };

    let payload = json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": std::process::id(),
            "activity": {
                "details": details,
                "state": state_text,
                "timestamps": {
                    "start": chrono::Utc::now().timestamp(),
                },
            },
        },
        "nonce": format!("{}", chrono::Utc::now().timestamp_millis()),
    });

    if let Err(err) = write_frame(stream.as_mut(), 1, &payload) {
        log::debug!("Failed to update Discord Rich Presence: {err}");
        state.connected = false;
        state.stream = None;
    }
}

pub fn set_idle() {
    let Some(state) = STATE.get() else {
        return;
    };
    let mut state = state.lock();
    set_activity(&mut state, "Quartz Launcher", "In the launcher");
}

pub fn set_playing(instance_name: &str, loader: &str, minecraft_version: &str) {
    let Some(state) = STATE.get() else {
        return;
    };
    let mut state = state.lock();
    set_activity(
        &mut state,
        &format!("{loader} · {minecraft_version}"),
        instance_name,
    );
}
