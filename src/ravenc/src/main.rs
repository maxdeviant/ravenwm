use ravenwm_core::ipc;

fn main() {
    let socket = ipc::SocketPath::new();
    let mut ipc_client = ipc::Client::connect(&socket);

    ipc_client.send(&ipc::Message::CloseWindow);
}
