use ravenwm_core::ipc;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "ravenc")]
struct Args {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "snake_case")]
enum Command {
    MoveWindow { x: u32, y: u32 },
    CloseWindow,
}

#[paw::main]
fn main(args: Args) {
    let socket = ipc::SocketPath::new();
    let mut ipc_client = ipc::Client::connect(&socket);

    match args.command {
        Command::MoveWindow { x, y } => {
            ipc_client.send(&ipc::Message::MoveWindow { x, y });
        }
        Command::CloseWindow => {
            ipc_client.send(&ipc::Message::CloseWindow);
        }
    }
}
