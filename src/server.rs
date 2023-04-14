use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    io,
    net::TcpListener,
    sync::mpsc::channel,
    thread,
    time::Duration,
};

use super::tcp;

fn generate_hash(c: String) -> u64 {
    let mut hasher = DefaultHasher::new();
    c.hash(&mut hasher);
    hasher.finish()
}

pub struct Server<'a> {
    address: &'a str,
}

impl<'a> Server<'a> {
    pub fn new(address: &'a str) -> Self {
        Self { address }
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        let listener = TcpListener::bind(self.address)?;

        let (tx, rx) = channel::<tcp::TcpStreamWrapper>();
        thread::spawn(move || {
            let mut stream_pool: Vec<tcp::TcpStreamWrapper> = Vec::new();
            loop {
                match rx.recv_timeout(Duration::from_nanos(10)) {
                    Ok(stream) => {
                        stream_pool.push(stream);
                    }
                    _ => {}
                }

                let mut messagess_by_conn_id: HashMap<u64, String> = HashMap::new();
                for stream in stream_pool.iter_mut() {
                    let data = stream.read().unwrap_or_else(|_| "".to_string());
                    if data.len() > 0 {
                        messagess_by_conn_id.insert(stream.id, data);
                    }
                }

                for (stream_id, msg) in messagess_by_conn_id.iter() {
                    for stream in stream_pool.iter_mut() {
                        if *stream_id == stream.id {
                            continue;
                        }

                        stream
                            .write(msg.as_bytes())
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

                    let mut conn = tcp::TcpStreamWrapper::new(conn, generate_hash(remote_address));
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
