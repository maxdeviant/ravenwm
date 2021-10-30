use std::{io::Write, os::unix::net::UnixStream};

fn main() {
    println!("Hello, world from ravenc!");

    let socket_path = std::env::var("RAVENWM_SOCKET").expect("Failed to read RAVENWM_SOCKET");

    let mut stream =
        UnixStream::connect(&socket_path).expect(&format!("Failed to connect to {}", socket_path));

    stream
        .write_all(&"Hello from ravenc".as_bytes())
        .expect("Failed to write to socket")
}
