use std::{
    env,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::channel,
    thread,
    time::Duration,
};

const HELP_TEXT: &str = "Usage:\n    ./tchat server\n    ./tchat client";
const DEFAULT_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "8080";

struct Conn(TcpStream);

impl Clone for Conn {
    fn clone(&self) -> Conn {
        Conn(self.0.try_clone().unwrap())
    }
}

impl Conn {
    fn new(conn: TcpStream) -> Self {
        Self(conn)
    }

    fn configure(&mut self) -> Result<bool, io::Error> {
        match self.0.set_nonblocking(true) {
            Err(err) => Err(err),
            Ok(_) => match self.0.set_nodelay(true) {
                Err(err) => Err(err),
                Ok(_) => Ok(true),
            },
        }
    }

    fn read(&mut self) -> String {
        let mut buff = String::new();
        self.0.read_to_string(&mut buff).unwrap_or_else(|_| 0);
        buff
    }

    fn write(&mut self, content: &String) {
        self.0
            .write_all(content.as_bytes())
            .unwrap_or_else(|err| println!("Error writing to buffer {err}"));
        self.0.flush().unwrap();
    }
}

struct Server<'a>(&'a str, Vec<Conn>);

impl<'a> Server<'a> {
    fn new(address: &'a str) -> Self {
        Self(address, Vec::new())
    }

    fn run(&mut self) {
        let listener = TcpListener::bind(self.0)
            .unwrap_or_else(|err| panic!("The following error happend {err}"));

        let (tx, rx) = channel::<Conn>();
        thread::spawn(move || {
            let mut conns: Vec<Conn> = Vec::new();
            loop {
                match rx.recv_timeout(Duration::from_nanos(10)) {
                    Ok(conn) => {
                        conns.push(conn);
                    }
                    _ => {}
                }

                let mut messagess: Vec<String> = Vec::new();
                for conn in conns.iter_mut() {
                    let data = conn.read();
                    messagess.push(data);
                }

                for conn in conns.iter_mut() {
                    for message in messagess.iter() {
                        conn.write(message);
                    }
                }

                messagess.clear();
            }
        });

        for c in listener.incoming() {
            match c {
                Ok(conn) => {
                    println!("Client with ip: {:?} connected", conn.peer_addr().unwrap());

                    let mut conn = Conn::new(conn);
                    if !conn.configure().unwrap_or_else(|err| {
                        println!("Error configuring the connection. {err}");
                        false
                    }) {
                        continue;
                    }

                    tx.send(conn).unwrap_or_else(|err| {
                        println!("Error happend trying to send client conn to thread. {err}");
                    });
                }
                Err(err) => {
                    println!("error: {err}")
                }
            }
        }
    }
}

struct Client<'a>(&'a str);

impl<'a> Client<'a> {
    fn new(address: &'a str) -> Self {
        Self(address)
    }

    fn connect(&self) {
        let address = self.0;
        match TcpStream::connect(address) {
            Ok(conn) => {
                let mut conn = Conn::new(conn);
                if !conn.configure().unwrap_or_else(|err| {
                    println!("Error configuring the connection. {err}");
                    false
                }) {
                    return;
                };

                let mut cloned_conn = conn.clone();
                thread::spawn(move || loop {
                    let buff = cloned_conn.read();
                    if buff.len() > 0 {
                        println!("<Server> {buff}");
                    }
                });

                loop {
                    print!(">>> ");
                    let mut in_buff = String::new();

                    io::stdin().read_line(&mut in_buff).unwrap_or_else(|err| {
                        println!("Error reading the standard input {err}.");
                        0
                    });

                    conn.write(&in_buff);
                }
            }
            Err(err) => {
                println!(
                    "Error trying to connect to {address} with the following error => '{err}'."
                );
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!("{HELP_TEXT}")
    } else {
        let subcommand = &args[1];
        let default_address = format!("{DEFAULT_ADDRESS}:{DEFAULT_PORT}");
        match subcommand.as_str() {
            "server" => {
                let mut server = Server::new(default_address.as_str());
                server.run()
            }
            "client" => Client::new(default_address.as_str()).connect(),
            _ => {
                println!("{HELP_TEXT}")
            }
        }
    }
}
