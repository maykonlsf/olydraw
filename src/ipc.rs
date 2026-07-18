use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::sync::mpsc::{Receiver, Sender, channel};

#[cfg(not(windows))]
use interprocess::local_socket::{GenericFilePath, ToFsName};
#[cfg(windows)]
use interprocess::local_socket::{GenericNamespaced, ToNsName};
use interprocess::local_socket::{ListenerOptions, Name, Stream, prelude::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcCommand {
    Toggle,
}

const CLOSE_MARKER: &str = "__close__";

/// Listening end of the single-instance socket. Dropping it closes the socket.
pub struct IpcServer {
    name: String,
    listener_thread: Option<std::thread::JoinHandle<()>>,
}

/// Binds the local socket `name` and spawns a listener thread that forwards
/// received commands to the returned channel. Fails if another live instance
/// already owns the socket.
pub fn bind(name: &str) -> Result<(IpcServer, Receiver<IpcCommand>), String> {
    let sock_name = socket_name(name)?;
    let listener = match ListenerOptions::new().name(sock_name.clone()).create_sync() {
        Ok(l) => l,
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            if Stream::connect(sock_name.clone()).is_ok() {
                return Err("another instance is already running".to_string());
            }
            // Stale socket left by a crashed instance: remove and retry.
            remove_stale_socket(name);
            ListenerOptions::new()
                .name(sock_name)
                .create_sync()
                .map_err(|e| e.to_string())?
        }
        Err(e) => return Err(e.to_string()),
    };

    let (tx, rx) = channel();
    let listener_thread = std::thread::spawn(move || listen(listener, tx));
    Ok((
        IpcServer {
            name: name.to_string(),
            listener_thread: Some(listener_thread),
        },
        rx,
    ))
}

/// Sends a command to the instance owning socket `name`.
pub fn send(name: &str, cmd: IpcCommand) -> Result<(), String> {
    let line = match cmd {
        IpcCommand::Toggle => "toggle\n",
    };
    send_line(name, line)
}

fn listen(listener: interprocess::local_socket::Listener, tx: Sender<IpcCommand>) {
    for conn in listener.incoming() {
        let Ok(conn) = conn else { continue };
        let mut line = String::new();
        if BufReader::new(conn).read_line(&mut line).is_err() {
            continue;
        }
        match line.trim() {
            "toggle" => {
                if tx.send(IpcCommand::Toggle).is_err() {
                    return;
                }
            }
            CLOSE_MARKER => return,
            _ => {}
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        // Unblock the accept loop so the listener is dropped and the socket freed.
        let _ = send_line(&self.name, "__close__\n");
        if let Some(handle) = self.listener_thread.take() {
            let _ = handle.join();
        }
        remove_stale_socket(&self.name);
    }
}

fn send_line(name: &str, line: &str) -> Result<(), String> {
    let sock_name = socket_name(name)?;
    let mut conn = Stream::connect(sock_name).map_err(|e| e.to_string())?;
    conn.write_all(line.as_bytes()).map_err(|e| e.to_string())
}

// On Unix an explicit filesystem path is used so a socket left behind by a
// killed process can be detected and removed; on Windows named pipes are
// kernel objects that vanish with the process, so the namespace is safe.
#[cfg(windows)]
fn socket_name(name: &str) -> Result<Name<'static>, String> {
    format!("{name}.sock")
        .to_ns_name::<GenericNamespaced>()
        .map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn socket_name(name: &str) -> Result<Name<'static>, String> {
    socket_path(name)
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn socket_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{name}.sock"))
}

fn remove_stale_socket(name: &str) {
    #[cfg(not(windows))]
    let _ = std::fs::remove_file(socket_path(name));
    #[cfg(windows)]
    let _ = name;
}
