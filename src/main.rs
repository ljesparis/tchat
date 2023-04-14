use std::env;
use tchat::{client::Client, server::Server};

const HELP_TEXT: &str = "Usage:\n    ./tchat server\n    ./tchat client";
const DEFAULT_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "8080";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!("{HELP_TEXT}")
    } else {
        let subcommand = &args[1];
        let default_address = format!("{DEFAULT_ADDRESS}:{DEFAULT_PORT}");
        match subcommand.as_str() {
            "server" => Server::new(default_address.as_str())
                .run()
                .unwrap_or_else(|err| println!("{err}")),
            "client" => Client::new(default_address.as_str())
                .connect()
                .unwrap_or_else(|err| println!("{err}")),
            _ => {
                println!("{HELP_TEXT}")
            }
        }
    }
}
