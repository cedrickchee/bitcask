use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

// use serde_json::Deserializer;

use crate::{KvsEngine, Result};

/// The server of a key value store.
pub struct KvsServer {
    // engine: E,
}

impl KvsServer {
    /// Create a `KvsServer` with a given storage engine.
    // pub fn new(engine: E) -> Self {
    //     Self { engine }
    // }

    /// Create a `KvsServer` with a given storage engine.
    pub fn new() -> Self {
        Self {}
    }

    /// Run the server listening on the given address
    pub fn run<A: ToSocketAddrs>(self, addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            debug!("Connection established");

            match stream {
                Ok(stream) => {
                    if let Err(e) = self.serve(stream) {
                        error!("Error on serving client: {}", e);
                    }
                }
                Err(e) => error!("Unable to connect: {}", e),
            }
        }

        Ok(())
    }

    fn serve(&self, tcp: TcpStream) -> Result<()> {
        let peer_addr = tcp.peer_addr()?;
        // let reader = BufReader::new(&tcp);
        // let mut writer = BufWriter::new(&tcp);
        // let req_reader = Deserializer::from_reader(reader);

        // ********** Reading request from the TCP stream **********
        let mut buffer = Vec::new();
        let mut reader = BufReader::new(&tcp);
        let mut writer = BufWriter::new(&tcp);
        while let Ok(read_bytes) = reader.read_until(b'\n', &mut buffer) {
            if read_bytes == 0 {
                break;
            }

            let request = String::from_utf8_lossy(&buffer);
            info!("Receive request from {}: {:?}", peer_addr, request);

            // ********** Writing response to the TCP stream **********
            let response = b"+PONG\r\n";
            // stream.write(response).expect("Response failed");
            writer.write(response)?;
            writer.flush()?;
            info!("Response sent to {}: {:?}", peer_addr, response);
        }

        Ok(())
    }
}
