mod message;

use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};

pub use message::*;

#[derive(Debug)]
pub struct SocketPath(String);

impl SocketPath {
    pub fn new() -> Self {
        let socket_path = std::env::var("RAVENWM_SOCKET").expect("Failed to read RAVENWM_SOCKET");

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
        // println!("accept");
        match self.listener.accept() {
            Ok((mut socket, _)) => {
                println!("Accepted");

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

    pub fn incoming(&self) -> IncomingMessages<'_> {
        IncomingMessages {
            listener: &self.listener,
        }
    }
}

pub struct IncomingMessages<'a> {
    listener: &'a UnixListener,
}

impl<'a> Iterator for IncomingMessages<'a> {
    type Item = Message;

    fn next(&mut self) -> Option<Message> {
        let stream = self.listener.into_iter().next()?;
        match stream {
            Ok(mut stream) => {
                let mut buffer = Vec::new();
                stream
                    .read_to_end(&mut buffer)
                    .expect("Failed to read message");

                let message: Message =
                    bincode::deserialize(&buffer).expect("Failed to deserialize message");

                Some(message)
            }
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.listener.into_iter().size_hint()
    }
}
