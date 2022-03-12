mod geometry;
mod plumage;

use std::os::unix::prelude::AsRawFd;

use nix::sys::select::{select, FdSet};
use ravenwm_core::ipc;
use xcb::{self, x, Xid};

use crate::{geometry::Rectangle, plumage::Color};

/// The event mask for the root window.
const ROOT_EVENT_MASK: x::EventMask = x::EventMask::SUBSTRUCTURE_REDIRECT
    .union(x::EventMask::SUBSTRUCTURE_NOTIFY)
    .union(x::EventMask::STRUCTURE_NOTIFY)
    .union(x::EventMask::BUTTON_PRESS);

fn main() -> xcb::Result<()> {
    let socket = ipc::SocketPath::new();
    let ipc_server = ipc::Server::bind(&socket);

    let (conn, preferred_screen) = xcb::Connection::connect(None).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(preferred_screen as usize).unwrap();

    conn.send_request_checked(&x::ChangeWindowAttributes {
        window: screen.root(),
        value_list: &[x::Cw::EventMask(ROOT_EVENT_MASK)],
    });

    let meta_window = conn.generate_id();

    conn.send_request(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: meta_window,
        parent: screen.root(),
        x: -1,
        y: -1,
        width: 1,
        height: 1,
        border_width: 0,
        class: x::WindowClass::InputOnly,
        visual: x::Window::none().resource_id(),
        value_list: &[],
    });

    let mut layout_mode = LayoutMode::Tiling;
    let mut clients: Vec<XClient> = Vec::new();

    let mut window_border_width = 0u32;
    let mut window_border_color = Color::MIDNIGHT_BLUE;

    let mut focused_client = None;

    let (wm_protocols, wm_delete_window) = {
        let wm_protocols_cookie = conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"WM_PROTOCOLS",
        });
        let wm_delete_window_cookie = conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"WM_DELETE_WINDOW",
        });

        (
            conn.wait_for_reply(wm_protocols_cookie)?,
            conn.wait_for_reply(wm_delete_window_cookie)?,
        )
    };

    let ipc_fd = ipc_server.as_raw_fd();
    let xcb_fd = conn.as_raw_fd();

    let mut descriptors = FdSet::new();

    'ravenwm: loop {
        conn.flush()?;

        descriptors.clear();
        descriptors.insert(ipc_fd);
        descriptors.insert(xcb_fd);

        let ready_fds = select(None, Some(&mut descriptors), None, None, None)
            .expect("Failed to read file descriptors");

        if ready_fds > 0 {
            if descriptors.contains(ipc_fd) {
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
                                    let event = x::ClientMessageEvent::new(
                                        currently_focused_client,
                                        wm_protocols.atom(),
                                        x::ClientMessageData::Data32([
                                            wm_delete_window.atom().resource_id(),
                                            x::CURRENT_TIME,
                                            0,
                                            0,
                                            0,
                                        ]),
                                    );

                                    println!("Sending WM_DELETE_WINDOW event");
                                    conn.send_request(&x::SendEvent {
                                        propagate: false,
                                        destination: x::SendEventDest::Window(
                                            currently_focused_client,
                                        ),
                                        event_mask: x::EventMask::NO_EVENT,
                                        event: &event,
                                    });
                                } else {
                                    println!("Killing client: {:?}", currently_focused_client);
                                    conn.send_request(&x::KillClient {
                                        resource: currently_focused_client.resource_id(),
                                    });
                                }

                                if let Some(currently_focused_index) = clients
                                    .iter()
                                    .position(|client| client.window() == currently_focused_client)
                                {
                                    clients.remove(currently_focused_index);
                                }

                                focused_client = clients.last().map(|client| client.window());
                            }
                        }
                        ipc::Message::MoveWindow { x, y } => {
                            if let Some(focused_window) = focused_client {
                                conn.send_request(&x::ConfigureWindow {
                                    window: focused_window,
                                    value_list: &[
                                        x::ConfigWindow::X(x as i32),
                                        x::ConfigWindow::Y(y as i32),
                                    ],
                                });
                            }
                        }
                        ipc::Message::SetBorderWidth { width } => {
                            window_border_width = width;

                            for client in &clients {
                                let window_geometry = {
                                    let cookie = conn.send_request(&x::GetGeometry {
                                        drawable: x::Drawable::Window(client.window()),
                                    });

                                    conn.wait_for_reply(cookie)?
                                };

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

                                conn.send_request(&x::ConfigureWindow {
                                    window: client.window(),
                                    value_list: &[
                                        x::ConfigWindow::BorderWidth(window_border_width),
                                        x::ConfigWindow::Width(window_dimensions.width as u32),
                                        x::ConfigWindow::Height(window_dimensions.height as u32),
                                    ],
                                });
                            }
                        }
                        ipc::Message::SetBorderColor { color } => {
                            window_border_color = Color::rgb(color.r, color.g, color.b);

                            for client in &clients {
                                conn.send_request(&x::ChangeWindowAttributes {
                                    window: client.window(),
                                    value_list: &[x::Cw::BorderPixel(window_border_color.into())],
                                });
                            }
                        }
                    }
                }
            }

            if descriptors.contains(xcb_fd) {
                while let Some(xcb::Event::X(event)) = conn.poll_for_event()? {
                    println!("Received event {:?}", event);

                    match event {
                        x::Event::MapRequest(map_request) => {
                            println!("XCB_MAP_REQUEST");

                            let client = XClient {
                                window: map_request.window(),
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
                                    conn.send_request(&x::ConfigureWindow {
                                        window: client.window(),
                                        value_list: &[
                                            x::ConfigWindow::X(window_dimensions.x as i32),
                                            x::ConfigWindow::Y(window_dimensions.y as i32),
                                            x::ConfigWindow::Width(window_dimensions.width as u32),
                                            x::ConfigWindow::Height(
                                                window_dimensions.height as u32,
                                            ),
                                            x::ConfigWindow::BorderWidth(window_border_width),
                                        ],
                                    });
                                }
                                LayoutMode::Stacking => {}
                            }

                            conn.send_request(&x::ChangeWindowAttributes {
                                window: client.window(),
                                value_list: &[x::Cw::BorderPixel(window_border_color.into())],
                            });

                            conn.send_request(&x::MapWindow {
                                window: client.window(),
                            });

                            focused_client = Some(client.window());

                            clients.push(client);
                        }
                        x::Event::ConfigureRequest(configure_request) => {
                            println!("XCB_CONFIGURE_REQUEST");

                            let mut values = Vec::with_capacity(7);

                            if configure_request
                                .value_mask()
                                .contains(x::ConfigWindowMask::X)
                            {
                                values.push(x::ConfigWindow::X(configure_request.x() as i32));
                            }

                            if configure_request
                                .value_mask()
                                .contains(x::ConfigWindowMask::Y)
                            {
                                values.push(x::ConfigWindow::Y(configure_request.y() as i32));
                            }

                            if configure_request
                                .value_mask()
                                .contains(x::ConfigWindowMask::WIDTH)
                            {
                                values
                                    .push(x::ConfigWindow::Width(configure_request.width() as u32));
                            }

                            if configure_request
                                .value_mask()
                                .contains(x::ConfigWindowMask::HEIGHT)
                            {
                                values.push(x::ConfigWindow::Height(
                                    configure_request.height() as u32
                                ));
                            }

                            if configure_request
                                .value_mask()
                                .contains(x::ConfigWindowMask::BORDER_WIDTH)
                            {
                                values.push(x::ConfigWindow::BorderWidth(
                                    configure_request.border_width() as u32,
                                ));
                            }

                            if configure_request
                                .value_mask()
                                .contains(x::ConfigWindowMask::SIBLING)
                            {
                                values.push(x::ConfigWindow::Sibling(configure_request.sibling()));
                            }

                            if configure_request
                                .value_mask()
                                .contains(x::ConfigWindowMask::STACK_MODE)
                            {
                                values.push(x::ConfigWindow::StackMode(
                                    configure_request.stack_mode(),
                                ));
                            }

                            let values = dbg!(values);

                            conn.send_request(&x::ConfigureWindow {
                                window: configure_request.window(),
                                value_list: values.as_slice(),
                            });

                            conn.send_request(&x::ChangeWindowAttributes {
                                window: configure_request.window(),
                                value_list: &[x::Cw::EventMask(
                                    x::EventMask::PROPERTY_CHANGE.union(x::EventMask::FOCUS_CHANGE),
                                )],
                            });
                        }
                        x::Event::MotionNotify(motion_notify) => {
                            println!("XCB_MOTION_NOTIFY");
                        }
                        x::Event::ButtonPress(button_press) => {
                            println!("Mouse button '{}' pressed", button_press.detail());

                            if button_press.detail() == 0x1 {
                                let child_window = button_press.child();

                                println!("Child window: {:?}", child_window);

                                if !child_window.is_none() {
                                    println!("Focusing window: {:?}", child_window);
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
        conn.send_request(&x::DestroyWindow {
            window: client.window(),
        });
    }

    conn.send_request(&x::DestroyWindow {
        window: meta_window,
    });

    conn.flush()?;

    Ok(())
}

#[derive(Debug)]
enum LayoutMode {
    Tiling,
    Stacking,
}

/// An X client.
#[derive(Debug)]
struct XClient {
    window: x::Window,
}

impl XClient {
    pub fn window(&self) -> x::Window {
        self.window
    }
}
