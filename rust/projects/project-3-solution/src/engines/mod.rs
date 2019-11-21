/// A key value storage engine interface called by KvsServer.
pub trait KvsEngine {}

mod kvs;

pub use self::kvs::KvStore;
