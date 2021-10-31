mod geometry;
mod plumage;

use std::os::unix::prelude::AsRawFd;

use nix::sys::select::{select, FdSet};
use ravenwm_core::ipc;
use xcb;

use crate::{geometry::Rectangle, plumage::Color};

/// The event mask for the root window.
const ROOT_EVENT_MASK: u32 = xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT
    | xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY
    | xcb::EVENT_MASK_STRUCTURE_NOTIFY
    | xcb::EVENT_MASK_BUTTON_PRESS;

fn main() {
    let socket = ipc::SocketPath::new();
    let ipc_server = ipc::Server::bind(&socket);

    let (conn, preferred_screen) = xcb::Connection::connect(None).unwrap();
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

    let mut layout_mode = LayoutMode::Tiling;
    let mut clients: Vec<XClient> = Vec::new();

    let mut window_border_width = 0u32;
    let mut window_border_color = Color::MIDNIGHT_BLUE;

    let mut focused_client = None;

    let (wm_protocols, wm_delete_window) = {
        let wm_protocols_cookie = xcb::intern_atom(&conn, false, "WM_PROTOCOLS");
        let wm_delete_window_cookie = xcb::intern_atom(&conn, false, "WM_DELETE_WINDOW");

        (
            wm_protocols_cookie.get_reply().unwrap().atom(),
            wm_delete_window_cookie.get_reply().unwrap().atom(),
        )
    };

    let ipc_fd = ipc_server.as_raw_fd();
    let xcb_fd = conn.as_raw_fd();

    let ipc_fd = dbg!(ipc_fd);
    let xcb_fd = dbg!(xcb_fd);

    let mut descriptors = FdSet::new();

    'ravenwm: loop {
        conn.flush();

        descriptors.clear();
        descriptors.insert(ipc_fd);
        descriptors.insert(xcb_fd);

        let ready_fds = select(
            descriptors.highest(),
            Some(&mut descriptors),
            None,
            None,
            None,
        )
        .expect("Failed to read file descriptors");

        if dbg!(ready_fds) > 0 {
            if dbg!(descriptors.contains(ipc_fd)) {
                if let Some(message) = ipc_server.accept() {
                    println!("Message: {:?}", message);

                    match message {
                        ipc::Message::Quit => {
                            println!("Quit");
                            break 'ravenwm;
                        }
                        ipc::Message::CloseWindow => {
                            if let Some(currently_focused_client) = focused_client {
                                let is_icccm = false;
                                if is_icccm {
                                    let wm_protocols = dbg!(wm_protocols);
                                    let wm_delete_window = dbg!(wm_delete_window);

                                    let event = xcb::ClientMessageEvent::new(
                                        32,
                                        currently_focused_client,
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
                                        currently_focused_client,
                                        xcb::EVENT_MASK_NO_EVENT,
                                        &event,
                                    );
                                } else {
                                    println!("Killing client: {}", currently_focused_client);
                                    xcb::kill_client(&conn, currently_focused_client);
                                }

                                if let Some(currently_focused_index) = clients
                                    .iter()
                                    .position(|client| client.id() == currently_focused_client)
                                {
                                    clients.remove(currently_focused_index);
                                }

                                focused_client = clients.last().map(|client| client.id());
                            }
                        }
                        ipc::Message::MoveWindow { x, y } => {
                            if let Some(focused_window) = focused_client {
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
                        ipc::Message::SetBorderWidth { width } => {
                            window_border_width = width;

                            for client in &clients {
                                let window_geometry =
                                    xcb::get_geometry(&conn, client.id()).get_reply().unwrap();

                                let current_border_width = window_geometry.border_width();

                                let border_width_delta =
                                    window_border_width as i32 - current_border_width as i32;

                                let mut window_dimensions = Rectangle::new(
                                    window_geometry.x(),
                                    window_geometry.y(),
                                    window_geometry.width(),
                                    window_geometry.height(),
                                );

                                window_dimensions.width -= 2 * border_width_delta as u16;
                                window_dimensions.height -= 2 * border_width_delta as u16;

                                xcb::configure_window(
                                    &conn,
                                    client.id(),
                                    &[
                                        (
                                            xcb::CONFIG_WINDOW_BORDER_WIDTH as u16,
                                            window_border_width,
                                        ),
                                        (
                                            xcb::CONFIG_WINDOW_WIDTH as u16,
                                            window_dimensions.width as u32,
                                        ),
                                        (
                                            xcb::CONFIG_WINDOW_HEIGHT as u16,
                                            window_dimensions.height as u32,
                                        ),
                                    ],
                                );
                            }
                        }
                        ipc::Message::SetBorderColor { color } => {
                            window_border_color = Color::rgb(color.r, color.g, color.b);

                            for client in &clients {
                                xcb::change_window_attributes(
                                    &conn,
                                    client.id(),
                                    &[(xcb::CW_BORDER_PIXEL, window_border_color.into())],
                                );
                            }
                        }
                    }
                }
            }

            if dbg!(descriptors.contains(xcb_fd)) {
                while let Some(event) = conn.poll_for_event() {
                    let response_type = event.response_type();

                    println!("Received event {}", response_type);

                    match response_type {
                        xcb::MAP_REQUEST => {
                            println!("XCB_MAP_REQUEST");

                            let map_request: &xcb::MapRequestEvent =
                                unsafe { xcb::cast_event(&event) };

                            let client = XClient {
                                id: map_request.window(),
                            };

                            let window_gap_width = 16u32;

                            let mut window_dimensions = Rectangle::new(
                                0,
                                0,
                                screen.width_in_pixels(),
                                screen.height_in_pixels(),
                            );

                            window_dimensions
                                .deflate(window_gap_width as i16, window_gap_width as i16);

                            window_dimensions.width -= 2 * window_border_width as u16;
                            window_dimensions.height -= 2 * window_border_width as u16;

                            match layout_mode {
                                LayoutMode::Tiling => {
                                    xcb::configure_window(
                                        &conn,
                                        client.id(),
                                        &[
                                            (
                                                xcb::CONFIG_WINDOW_X as u16,
                                                window_dimensions.x as u32,
                                            ),
                                            (
                                                xcb::CONFIG_WINDOW_Y as u16,
                                                window_dimensions.y as u32,
                                            ),
                                            (
                                                xcb::CONFIG_WINDOW_WIDTH as u16,
                                                window_dimensions.width as u32,
                                            ),
                                            (
                                                xcb::CONFIG_WINDOW_HEIGHT as u16,
                                                window_dimensions.height as u32,
                                            ),
                                            (
                                                xcb::CONFIG_WINDOW_BORDER_WIDTH as u16,
                                                window_border_width,
                                            ),
                                        ],
                                    );
                                }
                                LayoutMode::Stacking => {}
                            }

                            xcb::change_window_attributes(
                                &conn,
                                client.id(),
                                &[(xcb::CW_BORDER_PIXEL, window_border_color.into())],
                            );

                            xcb::map_window(&conn, client.id());

                            focused_client = Some(client.id());

                            clients.push(client);
                        }
                        xcb::CONFIGURE_REQUEST => {
                            println!("XCB_CONFIGURE_REQUEST");

                            let configure_request: &xcb::ConfigureRequestEvent =
                                unsafe { xcb::cast_event(&event) };

                            let mut values = Vec::with_capacity(7);

                            if configure_request.value_mask() & (xcb::CONFIG_WINDOW_X as u16) == 0 {
                                values.push((
                                    xcb::CONFIG_WINDOW_X as u16,
                                    configure_request.x() as u32,
                                ));
                            }

                            if configure_request.value_mask() & (xcb::CONFIG_WINDOW_Y as u16) == 0 {
                                values.push((
                                    xcb::CONFIG_WINDOW_Y as u16,
                                    configure_request.y() as u32,
                                ));
                            }

                            if configure_request.value_mask() & (xcb::CONFIG_WINDOW_WIDTH as u16)
                                == 0
                            {
                                values.push((
                                    xcb::CONFIG_WINDOW_WIDTH as u16,
                                    configure_request.width() as u32,
                                ));
                            }

                            if configure_request.value_mask() & (xcb::CONFIG_WINDOW_HEIGHT as u16)
                                == 0
                            {
                                values.push((
                                    xcb::CONFIG_WINDOW_HEIGHT as u16,
                                    configure_request.height() as u32,
                                ));
                            }

                            if configure_request.value_mask()
                                & (xcb::CONFIG_WINDOW_BORDER_WIDTH as u16)
                                == 0
                            {
                                values.push((
                                    xcb::CONFIG_WINDOW_BORDER_WIDTH as u16,
                                    configure_request.border_width() as u32,
                                ));
                            }

                            if configure_request.value_mask() & (xcb::CONFIG_WINDOW_SIBLING as u16)
                                == 0
                            {
                                values.push((
                                    xcb::CONFIG_WINDOW_SIBLING as u16,
                                    configure_request.sibling(),
                                ));
                            }

                            if configure_request.value_mask()
                                & (xcb::CONFIG_WINDOW_STACK_MODE as u16)
                                == 0
                            {
                                values.push((
                                    xcb::CONFIG_WINDOW_STACK_MODE as u16,
                                    configure_request.stack_mode() as u32,
                                ));
                            }

                            let values = dbg!(values);

                            xcb::configure_window(
                                &conn,
                                configure_request.window(),
                                values.as_slice(),
                            );

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
                        xcb::BUTTON_PRESS => {
                            let button_press: &xcb::ButtonPressEvent =
                                unsafe { xcb::cast_event(&event) };

                            println!("Mouse button '{}' pressed", button_press.detail());

                            if button_press.detail() == 0x1 {
                                let child_window = button_press.child();

                                println!("Child window: {}", child_window);

                                if child_window != xcb::NONE {
                                    println!("Focusing window: {}", child_window);
                                    focused_client = Some(child_window);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    for client in clients {
        xcb::destroy_window(&conn, client.id);
    }

    xcb::destroy_window(&conn, meta_window);

    conn.flush();
}

#[derive(Debug)]
enum LayoutMode {
    Tiling,
    Stacking,
}

/// An X client.
#[derive(Debug)]
struct XClient {
    id: xcb::Window,
}

impl XClient {
    pub fn id(&self) -> xcb::Window {
        self.id
    }
}
