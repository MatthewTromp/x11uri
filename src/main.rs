mod common;
mod server;
mod client;

use std::error::Error;

use clap::{Parser, Subcommand};
use simple_error::bail;
use x11uri::client::transmit_or_open;

use crate::client::UriSender;
use crate::server::main_loop;


#[derive(Debug, Parser)]
#[command(name = "x11uri")]
#[command(about = "Client/server for opening URIs over X11", long_about = None)]
struct MainCli {
    #[command(subcommand)]
    command: Modes,
}

#[derive(Debug, Subcommand)]
enum Modes {
    // Server
    Server,
    Client {
        #[arg(value_name = "URI")]
        uri: String
    }    
}

#[derive(Parser)]
struct AliasCli {
    uri: String,
}

fn main() -> Result<(), Box<(dyn Error + 'static)>> {
    // Extract the name that this was called with
    let name = called_name();
    // We have special behaviour when our name is the name of a URI opening program
    // so that this binary can be directly substituted for xdg-open and the like.
    match name.as_str() {
        "xdg-open" | "gnome-open" | "kde-open" | "wslview" => {
            // TODO: avoid recursively calling your own client forever???
            let args = AliasCli::parse();
            transmit_or_open(&args.uri)
        }
        // TODO: gio support (don't want to override all commands, just open)
        "gio" => {
            bail!("gio is not supported!")
        }
        _ => {
            let args = MainCli::parse();

            match args.command {
                Modes::Server => {
                    main_loop()
                }
                Modes::Client { uri } => {
                    let sender = UriSender::new()?;
                    sender.transmit_uri(&uri)
                }
            }
        }       
    }   
}


fn called_name() -> String {
    let arg0 = std::env::args().next().unwrap();
    let index = arg0.chars().enumerate().filter(|(_, chr)| *chr == '/').last().map(|(i, _)| i+1).unwrap_or(0);
    arg0.chars().skip(index).collect()
}
