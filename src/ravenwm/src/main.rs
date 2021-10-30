use std::{
    fs,
    io::{self, Read},
    os::unix::net::UnixListener,
};
use xcb;

fn main() {
    println!("Hello, world from ravenwm!");

    let socket_path = std::env::var("RAVENWM_SOCKET").expect("Failed to read RAVENWM_SOCKET");

    match fs::remove_file(&socket_path) {
        Ok(()) => {}
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                // Nothing to do, since the file does not exist.
            } else {
                panic!("{}", err);
            }
        }
    }

    let listener =
        UnixListener::bind(&socket_path).expect(&format!("Failed to connect to {}", socket_path));

    let (conn, preferred_screen) = xcb::Connection::connect(Some(":1")).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(preferred_screen as usize).unwrap();

    let window = conn.generate_id();

    let values = [
        (xcb::CW_BACK_PIXEL, screen.white_pixel()),
        (
            xcb::CW_EVENT_MASK,
            xcb::EVENT_MASK_EXPOSURE | xcb::EVENT_MASK_KEY_PRESS,
        ),
    ];

    xcb::create_window(
        &conn,
        xcb::COPY_FROM_PARENT as u8,
        window,
        screen.root(),
        0,
        0,
        150,
        150,
        10,
        xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
        screen.root_visual(),
        &values,
    );

    xcb::map_window(&conn, window);

    let title = "Basic Window";
    xcb::change_property(
        &conn,
        xcb::PROP_MODE_REPLACE as u8,
        window,
        xcb::ATOM_WM_NAME,
        xcb::ATOM_STRING,
        8,
        title.as_bytes(),
    );

    println!("Flushing connection");

    conn.flush();

    loop {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut message = String::new();
                    stream
                        .read_to_string(&mut message)
                        .expect("Faield to read message");

                    println!("Message: {}", message);
                }
                Err(err) => {
                    println!("{}", err);
                }
            }
        }

        if let Some(event) = conn.wait_for_event() {
        } else {
            break;
        }
    }
}
