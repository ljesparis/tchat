use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    io,
    net::TcpListener,
    sync::mpsc::{channel, Sender},
    thread,
    time::Duration,
};

use crate::tcp::TcpStreamWrapper;


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
        let tx = self.start_background();
        for incoming_stream in listener.incoming() {
            match incoming_stream {
                Ok(stream) => {
                    let remote_address = stream.peer_addr()?;
                    let remote_address =
                        format!("{}:{}", remote_address.ip(), remote_address.port());
                    println!("Client with ip: {remote_address} connected.");

                    let mut conn =
                        TcpStreamWrapper::new(stream, generate_hash(remote_address));
                    if let Err(err) = conn.configure() {
                        println!("Unable to configure stream -> '{err}'.");
                        println!("Trying to shutdown stream.");
                        conn.shutdown_conn().unwrap_or_else(|_| {});
                        continue;
                    }

                    tx.send(conn).unwrap_or_else(|err| {
                        println!("Unable to send stream to pool -> '{err}'.");
                    });
                }
                Err(err) => {
                    println!("Unkown error -> '{err}'.")
                }
            }
        }

        Ok(())
    }

    fn start_background(&self) -> Sender<TcpStreamWrapper> {
        let (tx, rx) = channel::<TcpStreamWrapper>();
        thread::spawn(move || {
            let mut stream_pool: Vec<TcpStreamWrapper> = Vec::new();
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

        tx
    }
}
