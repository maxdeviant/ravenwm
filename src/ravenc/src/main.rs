use hex_color::HexColor;
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
    Quit,
    MoveWindow { x: u32, y: u32 },
    CloseWindow,
    BorderWidth { width_in_px: u32 },
    BorderColor { color: HexColor },
}

#[paw::main]
fn main(args: Args) {
    let socket = ipc::SocketPath::new();
    let mut ipc_client = ipc::Client::connect(&socket);

    match args.command {
        Command::Quit => {
            ipc_client.send(&ipc::Message::Quit);
        }
        Command::MoveWindow { x, y } => {
            ipc_client.send(&ipc::Message::MoveWindow { x, y });
        }
        Command::CloseWindow => {
            ipc_client.send(&ipc::Message::CloseWindow);
        }
        Command::BorderWidth { width_in_px } => {
            ipc_client.send(&ipc::Message::SetBorderWidth { width: width_in_px });
        }
        Command::BorderColor { color } => {
            ipc_client.send(&ipc::Message::SetBorderColor { color });
        }
    }
}
