use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

use serde_json::Deserializer;

use crate::common::{GetResponse, RemoveResponse, Request, SetResponse};
use crate::{KvsEngine, Result};

/// The server of a key value store.
pub struct KvsServer<E: KvsEngine> {
    engine: E,
}

impl<E: KvsEngine> KvsServer<E> {
    /// Create a `KvsServer` with a given storage engine.
    pub fn new(engine: E) -> Self {
        Self { engine }
    }

    /// Run the server listening on the given address
    pub fn run<A: ToSocketAddrs>(mut self, addr: A) -> Result<()> {
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

    fn serve(&mut self, tcp: TcpStream) -> Result<()> {
        let peer_addr = tcp.peer_addr()?;
        let reader = BufReader::new(&tcp);
        let mut writer = BufWriter::new(&tcp);
        let req_reader = Deserializer::from_reader(reader).into_iter::<Request>();

        macro_rules! send_resp {
            ($resp:expr) => {{
                let resp = $resp;
                serde_json::to_writer(&mut writer, &resp)?;
                writer.flush()?;
                info!("Response sent to {}: {:?}", peer_addr, resp);
            };};
        }

        for request in req_reader {
            let req = request?;
            info!("Received request from {}: {:?}", peer_addr, req);

            match req {
                Request::Set { key, value } => {
                    let engine_response = match self.engine.set(key, value) {
                        Ok(_) => SetResponse::Ok(()),
                        Err(err) => SetResponse::Err(format!("{}", err)),
                    };
                    send_resp!(engine_response);
                }
                Request::Get { key } => {
                    let engine_response = match self.engine.get(key) {
                        Ok(value) => GetResponse::Ok(value),
                        Err(err) => GetResponse::Err(format!("{}", err)),
                    };
                    send_resp!(engine_response);
                }
                Request::Remove { key } => {
                    let engine_response = match self.engine.remove(key) {
                        Ok(_) => RemoveResponse::Ok(()),
                        Err(err) => RemoveResponse::Err(format!("{}", err)),
                    };
                    send_resp!(engine_response);
                }
            }
        }

        Ok(())
    }
}
