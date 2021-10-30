use ravenwm_core::ipc;
use xcb;

/// The event mask for the root window.
const ROOT_EVENT_MASK: u32 = xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT
    | xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY
    | xcb::EVENT_MASK_STRUCTURE_NOTIFY
    | xcb::EVENT_MASK_BUTTON_PRESS;

fn main() {
    let socket = ipc::SocketPath::new();
    let ipc_server = ipc::Server::bind(&socket);

    let (conn, preferred_screen) = xcb::Connection::connect(Some(":1")).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(preferred_screen as usize).unwrap();

    xcb::change_window_attributes_checked(
        &conn,
        screen.root(),
        &[(xcb::CW_EVENT_MASK, ROOT_EVENT_MASK)],
    );

    let meta_window = conn.generate_id();

    xcb::create_window(
        &conn,
        xcb::COPY_FROM_PARENT as u8,
        meta_window,
        screen.root(),
        -1,
        -1,
        1,
        1,
        0,
        xcb::WINDOW_CLASS_INPUT_ONLY as u16,
        xcb::NONE,
        &[],
    );

    let create_test_window = || {
        let test_window = conn.generate_id();

        xcb::create_window(
            &conn,
            xcb::COPY_FROM_PARENT as u8,
            test_window,
            screen.root(),
            0,
            0,
            150,
            150,
            10,
            xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
            screen.root_visual(),
            &[
                (xcb::CW_BACK_PIXEL, screen.white_pixel()),
                (
                    xcb::CW_EVENT_MASK,
                    xcb::EVENT_MASK_EXPOSURE | xcb::EVENT_MASK_KEY_PRESS,
                ),
            ],
        );

        xcb::map_window(&conn, test_window);

        test_window
    };

    let test_window = create_test_window();
    let test_window_2 = create_test_window();

    let mut focused_window = Some(test_window);

    let title = "Basic Window";
    xcb::change_property(
        &conn,
        xcb::PROP_MODE_REPLACE as u8,
        test_window,
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

    'ravenwm: loop {
        conn.flush();

        if let Some(message) = ipc_server.accept() {
            println!("Message: {:?}", message);

            match message {
                ipc::Message::Quit => {
                    println!("Quit");
                    break 'ravenwm;
                }
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
                            test_window,
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
                        xcb::send_event(
                            &conn,
                            false,
                            test_window,
                            xcb::EVENT_MASK_NO_EVENT,
                            &event,
                        );
                    } else {
                        println!("Killing client: {}", test_window);
                        xcb::kill_client(&conn, test_window);
                    }
                }
                ipc::Message::MoveWindow { x, y } => {
                    if let Some(focused_window) = focused_window {
                        xcb::configure_window(
                            &conn,
                            focused_window,
                            &[
                                (xcb::CONFIG_WINDOW_X as u16, x),
                                (xcb::CONFIG_WINDOW_Y as u16, y),
                            ],
                        );
                    }
                }
            }
        }

        while let Some(event) = conn.poll_for_event() {
            let response_type = event.response_type();

            println!("Received event {}", response_type);

            match response_type {
                xcb::MAP_REQUEST => {
                    println!("XCB_MAP_REQUEST");

                    let map_request: &xcb::MapRequestEvent = unsafe { xcb::cast_event(&event) };

                    xcb::map_window(&conn, map_request.window());
                }
                xcb::CONFIGURE_REQUEST => {
                    println!("XCB_CONFIGURE_REQUEST");

                    let configure_request: &xcb::ConfigureRequestEvent =
                        unsafe { xcb::cast_event(&event) };

                    let mut values = Vec::with_capacity(7);

                    if configure_request.value_mask() & (xcb::CONFIG_WINDOW_X as u16) == 0 {
                        values.push((xcb::CONFIG_WINDOW_X as u16, configure_request.x() as u32));
                    }

                    if configure_request.value_mask() & (xcb::CONFIG_WINDOW_Y as u16) == 0 {
                        values.push((xcb::CONFIG_WINDOW_Y as u16, configure_request.y() as u32));
                    }

                    if configure_request.value_mask() & (xcb::CONFIG_WINDOW_WIDTH as u16) == 0 {
                        values.push((
                            xcb::CONFIG_WINDOW_WIDTH as u16,
                            configure_request.width() as u32,
                        ));
                    }

                    if configure_request.value_mask() & (xcb::CONFIG_WINDOW_HEIGHT as u16) == 0 {
                        values.push((
                            xcb::CONFIG_WINDOW_HEIGHT as u16,
                            configure_request.height() as u32,
                        ));
                    }

                    if configure_request.value_mask() & (xcb::CONFIG_WINDOW_BORDER_WIDTH as u16)
                        == 0
                    {
                        values.push((
                            xcb::CONFIG_WINDOW_BORDER_WIDTH as u16,
                            configure_request.border_width() as u32,
                        ));
                    }

                    if configure_request.value_mask() & (xcb::CONFIG_WINDOW_SIBLING as u16) == 0 {
                        values.push((
                            xcb::CONFIG_WINDOW_SIBLING as u16,
                            configure_request.sibling(),
                        ));
                    }

                    if configure_request.value_mask() & (xcb::CONFIG_WINDOW_STACK_MODE as u16) == 0
                    {
                        values.push((
                            xcb::CONFIG_WINDOW_STACK_MODE as u16,
                            configure_request.stack_mode() as u32,
                        ));
                    }

                    let values = dbg!(values);

                    xcb::configure_window(&conn, configure_request.window(), values.as_slice());

                    xcb::change_window_attributes(
                        &conn,
                        configure_request.window(),
                        &[(
                            xcb::CW_EVENT_MASK,
                            xcb::EVENT_MASK_PROPERTY_CHANGE | xcb::EVENT_MASK_FOCUS_CHANGE,
                        )],
                    );
                }
                xcb::MOTION_NOTIFY => {
                    println!("XCB_MOTION_NOTIFY");

                    let _motion_notify: &xcb::MotionNotifyEvent =
                        unsafe { xcb::cast_event(&event) };
                }
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
                                test_window,
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
                            xcb::send_event(
                                &conn,
                                false,
                                test_window,
                                xcb::EVENT_MASK_NO_EVENT,
                                &event,
                            );
                        } else {
                            println!("Killing client: {}", test_window);
                            xcb::kill_client(&conn, test_window);
                        }
                    }
                }
                xcb::BUTTON_PRESS => {
                    let button_press: &xcb::ButtonPressEvent = unsafe { xcb::cast_event(&event) };

                    println!("Mouse button '{}' pressed", button_press.detail());

                    if button_press.detail() == 0x1 {
                        let child_window = button_press.child();

                        println!("Child window: {}", child_window);

                        if child_window != xcb::NONE {
                            println!("Focusing window: {}", child_window);
                            focused_window = Some(child_window);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    xcb::destroy_window(&conn, test_window);
    xcb::destroy_window(&conn, test_window_2);
    xcb::destroy_window(&conn, meta_window);

    conn.flush();
}
