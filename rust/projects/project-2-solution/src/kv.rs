use std::collections::HashMap;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use crate::Result;

/// The `KvStore` stores string key/value pairs.
///
/// Key/value pairs are stored in a `HashMap` in memory and not persisted to disk.
///
/// Example:
///
/// ```rust
/// # use kvs::KvStore;
/// let mut store = KvStore::new();
/// store.set(String::from("my_key"), String::from("my_value"));
///
/// let val = store.get(String::from("my_key"));
/// assert_eq!(val, Some(String::from("my_value")));
/// ```
pub struct KvStore {
    /// Directory for WAL and other data
    #[allow(dead_code)]
    path: PathBuf,
    map: HashMap<String, String>,
    writer: BufWriter<File>,
}

impl KvStore {
    /// Opens the store with the given path.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        create_dir_all(&path)?;

        let mut log_path = path.clone();
        log_path.push("kvs.log");
        let log = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&log_path)?;

        Ok(Self {
            path,
            writer: BufWriter::new(log),
            map: Self::load_from_log(&log_path)?,
        })
    }

    /// Set a given key and value Strings in the store.
    ///
    /// If the key already exists, the previous value will be overwritten.
    ///
    /// # Example
    ///
    /// ```
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::new();
    /// store.set(String::from("my_key"), String::from("my_value"));
    /// ```
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let command = Command::new(CommandType::Set, key.to_owned(), value.to_owned());
        serde_json::to_writer(&mut self.writer, &command)?;
        self.map.insert(key, value);
        Ok(())
    }

    /// Get a value from the store using a key String.
    ///
    /// Returns `None` if the given key does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// use kvs::KvStore;
    ///
    /// let store = KvStore::new();
    /// match store.get(String::from("my_key")) {
    ///     Some(value) => println!("Value: {}", value),
    ///     None => println!("Key not found"),
    /// }
    /// ```
    pub fn get(&self, key: String) -> Result<Option<String>> {
        Ok(self.map.get(&key).cloned())
    }

    /// Remove a given key from the store.
    ///
    /// # Example
    ///
    /// ```
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::new();
    /// store.remove(String::from("my_key"));
    /// ```
    pub fn remove(&mut self, key: String) -> Result<Option<String>> {
        let command = Command::new(CommandType::Remove, key.to_owned(), "".to_string());
        serde_json::to_writer(&mut self.writer, &command)?;

        Ok(self.map.remove(&key))
    }

    /// Load from the Write Ahead Log file.
    fn load_from_log(log_path: impl AsRef<Path>) -> Result<HashMap<String, String>> {
        let mut map = HashMap::new();
        let reader = BufReader::new(File::open(log_path)?);
        let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();
        while let Some(cmd) = stream.next() {
            match cmd? {
                Command {
                    cmd: CommandType::Set,
                    key,
                    value,
                } => {
                    map.insert(key, value);
                }
                Command {
                    cmd: CommandType::Remove,
                    key,
                    value,
                } => {
                    let _val = value; // FIXME: field value is not applicable for remove command
                    map.remove(&key);
                }
            }
        }

        Ok(map)
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum CommandType {
    Set,
    Remove,
}

/// Struct representing a command
#[derive(Serialize, Deserialize, Debug)]
struct Command {
    cmd: CommandType,
    key: String,
    value: String,
}

impl Command {
    fn new(cmd: CommandType, key: String, value: String) -> Command {
        Self { cmd, key, value }
    }
}
