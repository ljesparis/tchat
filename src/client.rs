use std::{io, net::TcpStream, thread};

use crate::tcp::TcpStreamWrapper;

pub struct Client<'a> {
    address: &'a str,
}

impl<'a> Client<'a> {
    pub fn new(address: &'a str) -> Self {
        Self { address }
    }

    pub fn connect(&self) -> Result<(), io::Error> {
        let address = self.address;
        let conn = TcpStream::connect(address)?;
        let mut conn = TcpStreamWrapper::new(conn, 0);

        let mut cloned_conn = conn.clone();
        thread::spawn(move || loop {
            let mut buff = cloned_conn.read().unwrap_or_else(|_| "".to_string());
            if buff.len() > 0 {
                buff.truncate(buff.len() - 1);
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
