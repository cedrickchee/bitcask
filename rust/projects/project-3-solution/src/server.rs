use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

use serde_json::Deserializer;

use crate::common::{GetResponse, RemoveResponse, Request, SetResponse};
use crate::Result;

/// The server of a key value store.
pub struct KvsServer {}

impl KvsServer {
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
                    debug!("key: {}, value: {}", key, value);
                    let engine_response = SetResponse::Ok(());
                    send_resp!(engine_response);
                }
                Request::Get { key } => {
                    debug!("key: {}", key);
                    let engine_response = GetResponse::Ok(Some("value42".to_string()));
                    send_resp!(engine_response);
                }
                Request::Remove { key } => {
                    debug!("key: {}", key);
                    let engine_response = RemoveResponse::Ok(());
                    send_resp!(engine_response);
                }
            }
        }

        Ok(())
    }
}
