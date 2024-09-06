use clap::Parser;

use x11uri::client::UriSender;

#[derive(Parser)]
struct Cli {
    uri: String,
}

fn main() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let args = Cli::parse();

    let sender = UriSender::new()?;
    sender.transmit_uri(&args.uri)
}
