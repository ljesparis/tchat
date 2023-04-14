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

struct StreamPool {
    pool: Vec<TcpStreamWrapper>,
}

impl StreamPool {
    fn new() -> Self {
        Self { pool: Vec::new() }
    }

    fn add_strean(&mut self, stream: TcpStreamWrapper) {
        self.pool.push(stream)
    }

    fn read_message_from_nodes(&mut self) -> HashMap<u64, String> {
        let mut messagess_by_conn_id: HashMap<u64, String> = HashMap::new();
        for stream in self.pool.iter_mut() {
            let data = stream.read().unwrap_or_else(|_| "".to_string());
            if data.len() > 0 {
                messagess_by_conn_id.insert(stream.id, data);
            }
        }

        messagess_by_conn_id
    }

    fn broadcast_message(&mut self, messages_by_stream_id: HashMap<u64, String>) {
        for (stream_id, msg) in messages_by_stream_id.iter() {
            for stream in self.pool.iter_mut() {
                if *stream_id == stream.id {
                    continue;
                }

                stream
                    .write(msg.as_bytes())
                    .unwrap_or_else(|err| println!("{err}"));
            }
        }
    }
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
        let tx = self.start_pool_handler();
        for incoming_stream in listener.incoming() {
            match incoming_stream {
                Ok(stream) => {
                    let remote_address = stream.peer_addr()?;
                    let remote_address =
                        format!("{}:{}", remote_address.ip(), remote_address.port());
                    println!("Client with ip: {remote_address} connected.");

                    let mut conn = TcpStreamWrapper::new(stream, generate_hash(remote_address));
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

    fn start_pool_handler(&self) -> Sender<TcpStreamWrapper> {
        let (tx, rx) = channel::<TcpStreamWrapper>();
        thread::spawn(move || {
            let mut stream_pool = StreamPool::new();
            loop {
                if let Ok(stream) = rx.recv_timeout(Duration::from_nanos(10)) {
                    stream_pool.add_strean(stream);
                }

                let messages_by_stream_id = stream_pool.read_message_from_nodes();
                stream_pool.broadcast_message(messages_by_stream_id)
            }
        });

        tx
    }
}
