use std::net::SocketAddr;

use tokio::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_serde_json::{ReadJson, WriteJson};

use crate::common::{Request, Response};
use crate::KvsError;

/// The client of a key value store.
pub struct KvsClient {
    tcp: Option<TcpStream>,
}

impl KvsClient {
    /// Connect to `addr` to access `KvsServer`.
    pub fn connect(addr: SocketAddr) -> impl Future<Item = Self, Error = KvsError> {
        TcpStream::connect(&addr)
            .map(|tcp| KvsClient { tcp: Some(tcp) })
            .map_err(|e| e.into())
    }

    /// Get a value from the server using a key String.
    pub fn get(
        mut self,
        key: String,
    ) -> impl Future<Item = (Option<String>, Self), Error = KvsError> {
        let tcp = self.tcp.take().unwrap();
        let write_json = WriteJson::new(FramedWrite::new(tcp, LengthDelimitedCodec::new()));
        let tcp = write_json
            .send(Request::Get { key })
            .map(|serialized| serialized.into_inner().into_inner());
        tcp.and_then(|tcp| {
            let read_json = ReadJson::new(FramedRead::new(tcp, LengthDelimitedCodec::new()));
            read_json.into_future().map_err(|(err, _)| err)
        })
        .map_err(|e| e.into())
        .and_then(move |(resp, read_json)| {
            self.tcp = Some(read_json.into_inner().into_inner());
            match resp {
                Some(Response::Get(value)) => Ok((value, self)),
                Some(Response::Err(msg)) => Err(KvsError::StringError(msg)),
                Some(_) => Err(KvsError::StringError("Invalid response".to_owned())),
                None => Err(KvsError::StringError("No response received".to_owned())),
            }
        })
    }

    /// Set a given key and value Strings in the server.
    pub fn set(mut self, key: String, value: String) -> impl Future<Item = Self, Error = KvsError> {
        let tcp = self.tcp.take().unwrap();
        let write_json = WriteJson::new(FramedWrite::new(tcp, LengthDelimitedCodec::new()));
        let tcp = write_json
            .send(Request::Set { key, value })
            .map(|serialized| serialized.into_inner().into_inner());
        tcp.and_then(|tcp| {
            let read_json = ReadJson::new(FramedRead::new(tcp, LengthDelimitedCodec::new()));
            read_json.into_future().map_err(|(err, _)| err)
        })
        .map_err(|e| e.into())
        .and_then(move |(resp, read_json)| {
            self.tcp = Some(read_json.into_inner().into_inner());
            match resp {
                Some(Response::Set) => Ok(self),
                Some(Response::Err(msg)) => Err(KvsError::StringError(msg)),
                Some(_) => Err(KvsError::StringError("Invalid response".to_owned())),
                None => Err(KvsError::StringError("No response received".to_owned())),
            }
        })
    }

    /// Remove a given key from the server.
    pub fn remove(mut self, key: String) -> impl Future<Item = Self, Error = KvsError> {
        let tcp = self.tcp.take().unwrap();
        let write_json = WriteJson::new(FramedWrite::new(tcp, LengthDelimitedCodec::new()));
        let tcp = write_json
            .send(Request::Remove { key })
            .map(|serialized| serialized.into_inner().into_inner());
        tcp.and_then(|tcp| {
            let read_json = ReadJson::new(FramedRead::new(tcp, LengthDelimitedCodec::new()));
            read_json.into_future().map_err(|(err, _)| err)
        })
        .map_err(|e| e.into())
        .and_then(move |(resp, read_json)| {
            self.tcp = Some(read_json.into_inner().into_inner());
            match resp {
                Some(Response::Remove) => Ok(self),
                Some(Response::Err(msg)) => Err(KvsError::StringError(msg)),
                Some(_) => Err(KvsError::StringError("Invalid response".to_owned())),
                None => Err(KvsError::StringError("No response received".to_owned())),
            }
        })
    }
}
