use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    env,
    hash::{Hash, Hasher},
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::channel,
    thread,
    time::Duration,
};

const HELP_TEXT: &str = "Usage:\n    ./tchat server\n    ./tchat client";
const DEFAULT_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "8080";

fn generate_hash(c: String) -> u64 {
    let mut hasher = DefaultHasher::new();
    c.hash(&mut hasher);
    hasher.finish()
}

struct Conn(TcpStream, Option<u64>);

impl Clone for Conn {
    fn clone(&self) -> Conn {
        Conn(self.0.try_clone().unwrap(), self.1)
    }
}

impl Conn {
    fn new(conn: TcpStream, id: Option<u64>) -> Self {
        Self(conn, id)
    }

    fn configure(&mut self) -> Result<(), io::Error> {
        self.0.set_nonblocking(true)?;
        self.0.set_nodelay(true)?;
        Ok(())
    }

    fn read(&mut self) -> Result<String, io::Error> {
        let mut buff = String::new();
        let mut reader = BufReader::new(&self.0);
        reader.read_line(&mut buff)?;
        Ok(buff)
    }

    fn write(&mut self, content: &[u8]) -> Result<(), io::Error> {
        self.0.write_all(content)?;
        self.0.flush()?;
        Ok(())
    }
}

struct Server<'a>(&'a str, Vec<Conn>);

impl<'a> Server<'a> {
    fn new(address: &'a str) -> Self {
        Self(address, Vec::new())
    }

    fn run(&mut self) -> Result<(), io::Error> {
        let listener = TcpListener::bind(self.0)?;

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

                let mut messagess_by_conn_id: HashMap<u64, String> = HashMap::new();
                for conn in conns.iter_mut() {
                    let data = conn.read().unwrap_or_else(|_| "".to_string());
                    if data.len() > 0 {
                        if let Some(id) = conn.1 {
                            messagess_by_conn_id.insert(id, data);
                        }
                    }
                }

                for (conn_id, msg) in messagess_by_conn_id.iter() {
                    for conn in conns.iter_mut() {
                        if let Some(id) = conn.1 {
                            if *conn_id == id {
                                continue;
                            }
                        }

                        conn.write(msg.as_bytes())
                            .unwrap_or_else(|err| println!("{err}"));
                    }
                }

                messagess_by_conn_id.clear();
            }
        });

        for c in listener.incoming() {
            match c {
                Ok(conn) => {
                    let remote_address = conn.peer_addr()?;
                    let remote_address =
                        format!("{}:{}", remote_address.ip(), remote_address.port());
                    println!("Client with ip: {remote_address} connected");

                    let mut conn = Conn::new(conn, Some(generate_hash(remote_address)));
                    if let Err(err) = conn.configure() {
                        println!("Error configuring the connection. {err}");
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

        Ok(())
    }
}

struct Client<'a>(&'a str);

impl<'a> Client<'a> {
    fn new(address: &'a str) -> Self {
        Self(address)
    }

    fn connect(&self) -> Result<(), io::Error> {
        let address = self.0;
        let conn = TcpStream::connect(address)?;
        let mut conn = Conn::new(conn, None);
        conn.configure()?;

        let mut cloned_conn = conn.clone();
        thread::spawn(move || loop {
            let buff = cloned_conn.read().unwrap_or_else(|_| "".to_string());
            if buff.len() > 0 {
                println!("<Server> {buff}");
            }
        });

        loop {
            let mut in_buff = String::new();
            io::stdin().read_line(&mut in_buff)?;
            conn.write(in_buff.as_bytes())?;
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
