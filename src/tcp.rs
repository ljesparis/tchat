use std::{
    io::{self, BufRead, BufReader, Write},
    net::TcpStream,
};


// i'm lazy with names
pub struct TcpStreamWrapper {
    stream: TcpStream,
    pub id: u64
} 


impl Clone for TcpStreamWrapper {
    fn clone(&self) -> Self {
        Self{
            stream: self.stream.try_clone().unwrap(),
            id: self.id
        }
    }
}

impl TcpStreamWrapper {
    pub fn new(stream: TcpStream, id: u64) -> Self {
        Self {stream, id }
    }

    pub fn configure(&mut self) -> Result<(), io::Error> {
        self.stream.set_nonblocking(true)?;
        self.stream.set_nodelay(true)?;
        Ok(())
    }

    pub fn read(&mut self) -> Result<String, io::Error> {
        let mut buff = String::new();
        let mut reader = BufReader::new(&self.stream);
        reader.read_line(&mut buff)?;
        Ok(buff)
    }

    pub fn write(&mut self, content: &[u8]) -> Result<(), io::Error> {
        self.stream.write_all(content)?;
        self.stream.flush()?;
        Ok(())
    }
}
