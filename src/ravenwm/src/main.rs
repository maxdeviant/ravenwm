use ravenwm_core::ipc;
use xcb;

fn main() {
    let socket = ipc::SocketPath::new();
    let ipc_server = ipc::Server::bind(&socket);

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

    let (wm_protocols, wm_delete_window) = {
        let wm_protocols_cookie = xcb::intern_atom(&conn, false, "WM_PROTOCOLS");
        let wm_delete_window_cookie = xcb::intern_atom(&conn, false, "WM_DELETE_WINDOW");

        (
            wm_protocols_cookie.get_reply().unwrap().atom(),
            wm_delete_window_cookie.get_reply().unwrap().atom(),
        )
    };

    loop {
        conn.flush();

        if let Some(message) = ipc_server.accept() {
            println!("Message: {:?}", message);

            match message {
                ipc::Message::Ping => {
                    println!("Pong")
                }
                ipc::Message::CloseWindow => {
                    let is_icccm = false;
                    if is_icccm {
                        let wm_protocols = dbg!(wm_protocols);
                        let wm_delete_window = dbg!(wm_delete_window);

                        let event = xcb::ClientMessageEvent::new(
                            32,
                            window,
                            wm_protocols,
                            xcb::ClientMessageData::from_data32([
                                wm_delete_window,
                                xcb::CURRENT_TIME,
                                0,
                                0,
                                0,
                            ]),
                        );

                        println!("Sending WM_DELETE_WINDOW event");
                        xcb::send_event(&conn, false, window, xcb::EVENT_MASK_NO_EVENT, &event);
                    } else {
                        println!("Killing client: {}", window);
                        xcb::kill_client(&conn, window);
                    }
                }
                ipc::Message::MoveWindow { x, y } => {
                    xcb::configure_window(
                        &conn,
                        window,
                        &[
                            (xcb::CONFIG_WINDOW_X as u16, x),
                            (xcb::CONFIG_WINDOW_Y as u16, y),
                        ],
                    );
                }
            }
        }

        while let Some(event) = conn.poll_for_event() {
            let response_type = event.response_type();

            println!("Received event {}", response_type);

            match response_type {
                xcb::KEY_PRESS => {
                    let key_press: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&event) };

                    println!("Key '{}' pressed", key_press.detail());

                    // Q
                    if key_press.detail() == 0x18 {
                        let is_icccm = false;
                        if is_icccm {
                            let wm_protocols = dbg!(wm_protocols);
                            let wm_delete_window = dbg!(wm_delete_window);

                            let event = xcb::ClientMessageEvent::new(
                                32,
                                window,
                                wm_protocols,
                                xcb::ClientMessageData::from_data32([
                                    wm_delete_window,
                                    xcb::CURRENT_TIME,
                                    0,
                                    0,
                                    0,
                                ]),
                            );

                            println!("Sending WM_DELETE_WINDOW event");
                            xcb::send_event(&conn, false, window, xcb::EVENT_MASK_NO_EVENT, &event);
                        } else {
                            println!("Killing client: {}", window);
                            xcb::kill_client(&conn, window);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
