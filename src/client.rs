use std::{error::Error, sync::mpsc::sync_channel, thread::{spawn, scope}, time::Duration};

use x11rb::{connection::Connection, rust_connection::RustConnection, protocol::{xproto::{Atom, WindowWrapper, PropMode, change_property, send_event, EventMask, CreateWindowAux, WindowClass, get_selection_owner, PropertyNotifyEvent, PROPERTY_NOTIFY_EVENT, Property, Window, destroy_window}, Event}, CURRENT_TIME, COPY_FROM_PARENT, COPY_DEPTH_FROM_PARENT, NONE};

use crate::common::{intern, SELECTION_STR, UTF8_STR, SIGNAL_ENV_VAR};

const PATH_ENV_VAR: &str = "PATH";

// The server sets an environment variable
// If the client finds that this environment variable is set, it doesn't trasmit again
// Instead it goes into special "find an xdg-open that doesn't open this client again"
// mode
pub fn transmit_or_open(uri: &str) -> Result<(), Box<dyn Error>> {
    match std::env::var(SIGNAL_ENV_VAR) {
        Err(_) => UriSender::new()?.transmit_uri(uri),
        Ok(_) => find_opening_program(uri)
    }
}

fn find_opening_program(uri: &str) -> Result<(), Box<dyn Error>> {
    // We're being called by the server (or there's no server running).
    // We need to find a real opening program.
    // Get the current path, skip the first component, and then try to open.
    // If we find this binary again, then just keep doing it until it works.

    // TODO: could be made more efficient by finding our current executable's
    // folder and just removing that?
    let path = std::env::var(PATH_ENV_VAR).expect("No path environment variable");
    let chars_to_skip = path
        .chars()
        .enumerate()
        .filter(|(_, c)| *c == ':')  // Find the first ':'
        .next()
        .map(|(i, _)| i+1)
        .ok_or(simple_error::SimpleError::new("No real opening program found"))?;

    let new_path: String = path.chars().skip(chars_to_skip).collect();
    std::env::set_var(PATH_ENV_VAR, new_path);
    Ok(open::that(uri)?)
}

const PROPERTY_STR: &str = "URI_PROP";

pub struct UriSender {
    conn: RustConnection,
    // Store atoms from string interning to save from having to do it more than once
    selection_atom: Atom,
    prop_atom: Atom,
    encoding_atom: Atom,
    // Storing window as raw identifier rather than WindowWrapper due to lifetime issues.
    // See impl drop below
    window: Window,
}

impl UriSender {
    pub fn new() -> Result<Self, Box<(dyn Error + 'static)>> {
        let (conn, screen_num) = RustConnection::connect(None)?;

        let screen = &conn.setup().roots[screen_num];
        let selection_atom = intern(&conn, SELECTION_STR)?;
        let prop_atom = intern(&conn, PROPERTY_STR)?;
        let encoding_atom = intern(&conn, UTF8_STR)?;

        // Create dummy window
        let mut aux = CreateWindowAux::new();
        aux.event_mask = Some(EventMask::PROPERTY_CHANGE);
        let window = WindowWrapper::create_window(
            &conn,
            COPY_DEPTH_FROM_PARENT,
            screen.root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            COPY_FROM_PARENT,
            &aux)?.into_window();

        Ok(UriSender {
            conn,
            selection_atom,
            prop_atom,
            encoding_atom,
            window,
        })
    }

    pub fn transmit_uri(&self, uri: &str) -> Result<(), Box<(dyn Error + 'static)>> {
        self.set_uri(&uri)?;

        // Wait for our property to be deleted by the URI handler program
        self.wait_for_property_deletion()?;
        Ok(())
    }
    
    fn set_uri(&self, uri: &str) -> Result<(), Box<(dyn Error + 'static)>> {
        // Find the current owner of the "URI" selection.
        // We don't actually use this selection. It's just an identifier that tells us where
        // to send our URI.
        let uri_handler = get_selection_owner(&self.conn, self.selection_atom)?.reply()?.owner;
        if uri_handler == NONE {
            // TODO: Maybe instead of failing, it should try to find a real opening program,
            // like when it's called by the server?
            eprintln!("No owner for the \"URI\" selection. Make sure the x11uri server is running!");
            return find_opening_program(uri);
        }
        
        // Set a property on our window. This is what the URI handler program will read.
        change_property(&self.conn, PropMode::REPLACE, self.window, self.prop_atom, self.encoding_atom, 8, uri.as_bytes().len() as u32, uri.as_bytes())?.check()?;
        
        // Inform the URI handler via a PropertyNotifyEvent.
        // This is supposed to be used to inform a process about changes in the properties of its
        // own windows, but I don't know of a more correct way of sending this information.
        send_event(&self.conn, false, uri_handler, EventMask::NO_EVENT, PropertyNotifyEvent {
            response_type: PROPERTY_NOTIFY_EVENT,
            sequence: 1, // TODO: Is this okay?
            time: CURRENT_TIME, // TODO: Pretty sure this isn't okay
            window: self.window,
            atom: self.prop_atom,
            state: Property::NEW_VALUE,
        })?.check()?;
        
        Ok(())
    }


    fn wait_for_property_deletion(&self) ->  Result<(), Box<(dyn Error + 'static)>> {
        // TODO: don't wait forever? Is that a concern?
        loop {
            let event = &self.conn.wait_for_event()?;
            match event {
                Event::PropertyNotify(pne) => {
                    if pne.state == Property::DELETE
                        && pne.window == self.window
                        && pne.atom == self.prop_atom {
                            break;
                        }
                }
                e => {eprintln!("Unexpected event: {:?}", e)}
            }
        }
        
        Ok(())
    }

    fn wait_for_property_deletion_with_timeout(&self) ->  Result<(), Box<(dyn Error + 'static)>> {        
        scope(|scope| {
            let (tx, rx) = sync_channel::<Result<(), Box<dyn Error + Send + Sync>>>(1);

            let _ = scope.spawn(move || {
                tx.send((|| {
                    loop {
                        let event = &self.conn.wait_for_event()?;
                        match event {
                            Event::PropertyNotify(pne) => {
                                if pne.state == Property::DELETE
                                    && pne.window == self.window
                                    && pne.atom == self.prop_atom {
                                        break Ok(());
                                    }
                            }
                            e => {eprintln!("Unexpected event: {:?}", e)}
                        }
                    }
                })())
            });

            rx.recv_timeout(Duration::from_secs(1))?.map_err(|e| {e as _})
        })
    }
}

// Because of lifetime stuff, we can't store the window as a WindowWrapper, so we
// re-implement the destroy-on-drop behaviour of WindowWrapper here
impl Drop for UriSender {
    fn drop(&mut self) {
        let _ = destroy_window(&self.conn, self.window);
    }
}
