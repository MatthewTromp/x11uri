use std::error::Error;

use x11rb::{rust_connection::RustConnection, protocol::xproto::{Atom, intern_atom}};

pub const SELECTION_STR: &str = "URI";
pub const UTF8_STR: &str = "UTF8_STRING";
pub const SIGNAL_ENV_VAR: &str = "X11URI_PATH_INDEX";

pub fn intern(conn: &RustConnection, string: &str) -> Result<Atom, Box<(dyn Error + 'static)>> {
    Ok(intern_atom(conn, false, string.as_bytes())?.reply()?.atom)
}
