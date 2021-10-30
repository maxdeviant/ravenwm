mod message;

use std::fs;
use std::io::{self, Read, Write};
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
    stream: UnixStream,
}

impl Client {
    pub fn connect(socket: &SocketPath) -> Self {
        let stream =
            UnixStream::connect(&socket.0).expect(&format!("Failed to connect to {}", socket.0));

        Self { stream }
    }

    pub fn send(&mut self, message: &Message) {
        let buffer = bincode::serialize(message).expect("Failed to serialize message");

        self.stream
            .write_all(&buffer)
            .expect("Failed to send message");
    }
}

pub struct Server {
    listener: UnixListener,
}

impl Server {
    pub fn bind(socket: &SocketPath) -> Self {
        socket.delete_if_exists().expect("Failed to delete socket");

        let listener =
            UnixListener::bind(&socket.0).expect(&format!("Failed to connect to {}", socket.0));

        Self { listener }
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
