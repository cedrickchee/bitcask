use std::collections::HashMap;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use crate::Result;

/// The `KvStore` stores string key/value pairs.
///
/// Key/value pairs are stored in a `HashMap` in memory for fast query
/// and also persisted to disk in a log.
///
/// Example:
///
/// ```rust
/// use std::env::current_dir;
/// use kvs::KvStore;
/// let mut store = KvStore::open(current_dir().unwrap()).unwrap();
/// store.set(String::from("my_key"), String::from("my_value")).unwrap();
///
/// let val = store.get(String::from("my_key")).unwrap();
/// assert_eq!(val, Some(String::from("my_value")));
/// ```
pub struct KvStore {
    /// Directory the log and other data
    #[allow(dead_code)]
    path: PathBuf,
    /// Writer of the log
    writer: BufWriterWithPos<File>,
    /// Reader of the log
    reader: BufReaderWithPos<File>,
    /// Stores keys and the pos of the last command
    index: HashMap<String, CommandPos>,
}

impl KvStore {
    /// Opens the store with the given path.
    ///
    /// # Error
    ///
    /// It propagates I/O or deserialization errors during the log replay.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        create_dir_all(&path)?;

        let log_path = path.join("kvs.log");
        let log = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&log_path)?;
        let mut log_writer = BufWriterWithPos::new(log)?;
        // Set pos to end of file
        log_writer.seek(SeekFrom::End(0))?;

        let log_reader = BufReaderWithPos::new(File::open(&log_path)?)?;

        let mut store = Self {
            path,
            writer: log_writer,
            reader: log_reader,
            index: HashMap::new(),
        };
        store.load_from_log()?;
        Ok(store)
    }

    /// Set a given key and value Strings in the store.
    ///
    /// If the key already exists, the previous value will be overwritten.
    ///
    /// # Error
    ///
    /// It propagates I/O or serialization errors during writing the log.
    ///
    /// # Example
    ///
    /// ```
    /// use std::env::current_dir;
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::open(current_dir().unwrap()).unwrap();
    /// store.set(String::from("my_key"), String::from("my_value")).unwrap();
    /// ```
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let command = Command::new(CommandType::Set, key.to_owned(), value.to_owned());
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, &command)?;
        self.writer.flush()?;
        self.index.insert(key, (pos..self.writer.pos).into());
        Ok(())
    }

    /// Get a value from the store using a key String.
    ///
    /// Returns `None` if the given key does not exist.
    ///
    /// # Example
    ///
    /// ```
    /// use std::env::current_dir;
    /// use kvs::KvStore;
    ///
    /// let store = KvStore::open(current_dir().unwrap()).unwrap();
    /// match store.get(String::from("my_key")).unwrap() {
    ///     Some(value) => println!("Value: {}", value),
    ///     None => println!("Key not found"),
    /// }
    /// ```
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.index.get(&key) {
            self.reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            let cmd_reader = (&mut self.reader).take(cmd_pos.len);
            let cmd: Command = serde_json::from_reader(cmd_reader)?;
            Ok(Some(cmd.value))
        } else {
            Ok(None)
        }
    }

    /// Remove a given key from the store.
    ///
    /// # Example
    ///
    /// ```
    /// use std::env::current_dir;
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::open(current_dir().unwrap()).unwrap();
    /// store.remove(String::from("my_key")).unwrap();
    /// ```
    pub fn remove(&mut self, key: String) -> Result<()> {
        let command = Command::new(CommandType::Remove, key.to_owned(), "".to_string());
        serde_json::to_writer(&mut self.writer, &command)?;
        self.writer.flush()?;

        self.index.remove(&key);

        Ok(())
    }

    /// Load from the log file.
    fn load_from_log(&mut self) -> Result<()> {
        let mut pos = self.reader.seek(SeekFrom::Start(0))?;
        let mut stream = Deserializer::from_reader(&mut self.reader).into_iter::<Command>();
        while let Some(cmd) = stream.next() {
            let new_pos = stream.byte_offset() as u64;
            match cmd? {
                Command {
                    cmd: CommandType::Set,
                    key,
                    value,
                } => {
                    let _val = value; // FIXME: field value is not applicable for remove command
                    self.index.insert(key, (pos..new_pos).into());
                }
                Command {
                    cmd: CommandType::Remove,
                    key,
                    value,
                } => {
                    let _val = value; // FIXME: field value is not applicable for remove command
                    self.index.remove(&key);
                }
            }

            pos = new_pos;
        }

        Ok(())
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

/// Represents the position and length of a JSON-serialized command in the log.
struct CommandPos {
    pos: u64,
    len: u64,
}

impl From<Range<u64>> for CommandPos {
    fn from(range: Range<u64>) -> Self {
        Self {
            pos: range.start,
            len: range.end - range.start,
        }
    }
}

/// A wrapper of BufReader of the log file
struct BufReaderWithPos<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> BufReaderWithPos<R> {
    fn new(mut inner: R) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufReaderWithPos {
            reader: BufReader::new(inner),
            pos,
        })
    }
}

impl<R: Read + Seek> Read for BufReaderWithPos<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

impl<R: Read + Seek> Seek for BufReaderWithPos<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.reader.seek(pos)?;
        Ok(self.pos)
    }
}

/// A wrapper of BufWriter of the log file
struct BufWriterWithPos<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl<W: Write + Seek> BufWriterWithPos<W> {
    fn new(mut inner: W) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(BufWriterWithPos {
            writer: BufWriter::new(inner),
            pos,
        })
    }
}

impl<W: Write + Seek> Write for BufWriterWithPos<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for BufWriterWithPos<W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}
