use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};

use serde_json::Deserializer;

use crate::common::{GetResponse, RemoveResponse, Request, SetResponse};
use crate::thread_pool::ThreadPool;
use crate::{KvsEngine, Result};

/// The server of a key value store.
pub struct KvsServer<E: KvsEngine, P: ThreadPool> {
    engine: E,
    thread_pool: P,
}

impl<E: KvsEngine, P: ThreadPool> KvsServer<E, P> {
    /// Create a `KvsServer` with a given storage engine.
    pub fn new(engine: E, thread_pool: P) -> Self {
        Self {
            engine,
            thread_pool,
        }
    }

    /// Run the server listening on the given address
    pub fn run<A: ToSocketAddrs>(self, addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            debug!("Connection established");

            let engine = self.engine.clone();

            self.thread_pool.spawn(move || match stream {
                Ok(stream) => {
                    if let Err(e) = serve(engine, stream) {
                        error!("Error on serving client: {}", e);
                    }
                }
                Err(e) => error!("Unable to connect: {}", e),
            })
        }

        Ok(())
    }
}

fn serve<E: KvsEngine>(engine: E, tcp: TcpStream) -> Result<()> {
    let peer_addr = tcp.peer_addr()?;
    let reader = BufReader::new(&tcp);
    let mut writer = BufWriter::new(&tcp);
    let req_reader = Deserializer::from_reader(reader).into_iter::<Request>();

    macro_rules! send_resp {
        ($resp:expr) => {{
            let resp = $resp;
            serde_json::to_writer(&mut writer, &resp)?;
            writer.flush()?;
            debug!("Response sent to {}: {:?}", peer_addr, resp);
        };};
    }

    for request in req_reader {
        let req = request?;
        debug!("Received request from {}: {:?}", peer_addr, req);

        match req {
            Request::Set { key, value } => {
                let engine_response = match engine.set(key, value) {
                    Ok(_) => SetResponse::Ok(()),
                    Err(err) => SetResponse::Err(format!("{}", err)),
                };
                send_resp!(engine_response);
            }
            Request::Get { key } => {
                let engine_response = match engine.get(key) {
                    Ok(value) => GetResponse::Ok(value),
                    Err(err) => GetResponse::Err(format!("{}", err)),
                };
                send_resp!(engine_response);
            }
            Request::Remove { key } => {
                let engine_response = match engine.remove(key) {
                    Ok(_) => RemoveResponse::Ok(()),
                    Err(err) => RemoveResponse::Err(format!("{}", err)),
                };
                send_resp!(engine_response);
            }
        }
    }

    Ok(())
}
