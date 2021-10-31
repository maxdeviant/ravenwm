mod message;

use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

pub use message::*;

#[derive(Debug)]
pub struct SocketPath(String);

impl SocketPath {
    pub fn new() -> Self {
        let socket_path = std::env::var("RAVENWM_SOCKET").unwrap_or_else(|_| {
            let xdg_runtime_dir =
                std::env::var("XDG_RUNTIME_DIR").expect("Failed to get XDG_RUNTIME_DIR");

            PathBuf::from(xdg_runtime_dir)
                .join("ravenwm.sock")
                .to_str()
                .expect("Invalid socket path")
                .to_string()
        });

        Self(socket_path)
    }

    fn delete_if_exists(&self) -> io::Result<()> {
        match fs::remove_file(&self.0) {
            Ok(()) => Ok(()),
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(err)
                }
            }
        }
    }
}

pub struct Client {
    socket: UnixStream,
}

impl Client {
    pub fn connect(socket_path: &SocketPath) -> Self {
        let socket = UnixStream::connect(&socket_path.0)
            .expect(&format!("Failed to connect to {}", socket_path.0));

        Self { socket }
    }

    pub fn send(&mut self, message: &Message) {
        let buffer = bincode::serialize(message).expect("Failed to serialize message");

        self.socket
            .write_all(&buffer)
            .expect("Failed to send message");
    }
}

pub struct Server {
    listener: UnixListener,
}

impl Server {
    pub fn bind(socket_path: &SocketPath) -> Self {
        socket_path
            .delete_if_exists()
            .expect("Failed to delete socket");

        let listener = UnixListener::bind(&socket_path.0)
            .expect(&format!("Failed to connect to {}", socket_path.0));

        listener.set_nonblocking(true).unwrap();

        Self { listener }
    }

    pub fn accept(&self) -> Option<Message> {
        match self.listener.accept() {
            Ok((mut socket, _)) => {
                let mut buffer = Vec::new();
                socket
                    .read_to_end(&mut buffer)
                    .expect("Failed to read message");

                let message: Message =
                    bincode::deserialize(&buffer).expect("Failed to deserialize message");

                Some(message)
            }
            Err(err) => {
                if err.kind() == ErrorKind::WouldBlock {
                    return None;
                }

                println!("Socket error: {}", err);
                None
            }
        }
    }
}
