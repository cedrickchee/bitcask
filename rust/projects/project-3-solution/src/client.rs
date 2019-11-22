use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};

use crate::Result;

/// The client of a key value store.
pub struct KvsClient {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
}

impl KvsClient {
    /// Connect to `addr` to access `KvsServer`.
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let tcp_reader = TcpStream::connect(addr)?;
        let tcp_writer = tcp_reader.try_clone()?;

        Ok(Self {
            reader: BufReader::new(tcp_reader),
            writer: BufWriter::new(tcp_writer),
        })
    }

    /// Get a value from the server using a key String.
    ///
    /// Returns `None` if the given key does not exist.
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        let request = format!("+GET,{}\n", key);
        self.writer.write(request.as_bytes())?;
        self.writer.flush()?;

        let mut response = String::new();
        let read_bytes = self.reader.read_line(&mut response)?;
        println!("Server response with {} bytes: {}", read_bytes, response);

        if response.is_empty() {
            Ok(None)
        } else {
            Ok(Some(response))
        }
    }

    /// Set a given key and value Strings in the server.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let request = format!("+SET,{},{}\n", key, value);
        self.writer.write(request.as_bytes())?;
        self.writer.flush()?;

        let mut response = String::new();
        let read_bytes = self.reader.read_line(&mut response)?;
        println!("Server response with {} bytes: {}", read_bytes, response);

        Ok(())
    }

    /// Remove a given key from the server.
    pub fn remove(&mut self, key: String) -> Result<()> {
        let request = format!("+REMOVE,{}\n", key);
        self.writer.write(request.as_bytes())?;
        self.writer.flush()?;

        let mut response = String::new();
        let read_bytes = self.reader.read_line(&mut response)?;
        println!("Server response with {} bytes: {}", read_bytes, response);

        Ok(())
    }
}
