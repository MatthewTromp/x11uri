use std::{error::Error, thread};

use x11rb::{rust_connection::{RustConnection, ReplyError}, protocol::{xproto::{WindowClass, CreateWindowAux, set_selection_owner, get_property, WindowWrapper, GetPropertyReply}, Event}, connection::Connection, CURRENT_TIME, COPY_FROM_PARENT, COPY_DEPTH_FROM_PARENT};

use crate::common::{intern, UTF8_STR, SELECTION_STR, SIGNAL_ENV_VAR};

const MAX_URI_LENGTH: u32 = u32::MAX;

pub fn main_loop() -> Result<(), Box<(dyn Error + 'static)>> {
    // Set an environment variable (in case we call ourself)
    std::env::set_var(SIGNAL_ENV_VAR, "0");

    // Establish our x connection
    let (conn, screen_num) = RustConnection::connect(None)?;
    
    let screen = &conn.setup().roots[screen_num];

    // Create dummy window to get responses
    let window = WindowWrapper::create_window(&conn, COPY_DEPTH_FROM_PARENT, screen.root, 0, 0, 1, 1, 0, WindowClass::INPUT_ONLY, COPY_FROM_PARENT, &CreateWindowAux::new())?;

    // Create and acquire selection
    let selection_atom = intern(&conn, SELECTION_STR)?;
    set_selection_owner(&conn, window.window(), selection_atom, CURRENT_TIME)?.check()?;

    let encoding_atom = intern(&conn, UTF8_STR)?;

    loop {
        let event = conn.wait_for_event()?;
        match event {
            Event::PropertyNotify(pne) => {
                if pne.window == window.window() {
                    eprintln!("Error: Recieved a PropertyNotify event for our own window: {:?}", pne);
                    continue;
                }

                // Someone has requested we open a URI at a given property
                
                // Get the value of this property
                let property_response = get_property(&conn, true, pne.window, pne.atom, encoding_atom, 0, MAX_URI_LENGTH).expect("Connection error!").reply();
                // Spawn a thread to handle opening the property
                spawn_uri_handler(property_response);
            }
            Event::SelectionClear(_) => {
                // Someone stole our selection (maybe another x11uri server instance)
                // Close to not be a resource leak
                panic!("Lost selection");
            }
            _ => {}
        }
    }
}

fn spawn_uri_handler(property_response: Result<GetPropertyReply, ReplyError>) {
    thread::spawn(|| {
        match property_response {
            Err(e) => eprintln!("Failed to get a reply: {e}"),
            Ok(reply) => match reply.value8() {
                None => eprintln!("Property with incorrect type: could not interpret as u8s"),
                Some(byte_iterator) => {
                    let bytestring: Vec<u8> = byte_iterator.collect();
                    match std::str::from_utf8(&bytestring) {
                        Err(e) => eprintln!("Error parsing URI: {e}"),
                        Ok(uri) => open_uri(uri).unwrap_or_else(|e| {
                            eprintln!("Error opening URI: {e}")
                        }),
                    };
                },
            }
        };
    });
}

fn open_uri(uri: &str) -> Result<(), Box<(dyn Error + 'static)>> {
//    println!("Opening URI: {uri}");
    Ok(open::that(uri)?)
}


