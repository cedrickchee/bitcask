use std::cmp;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::mem;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use crate::Result;

const COMPACTION_THRESHOLD: u64 = 1024;

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
    path: PathBuf,
    kv_log: KvLog,
}

impl KvStore {
    /// Opens the store with the given path.
    ///
    /// # Error
    ///
    /// It propagates I/O or deserialization errors during the log replay.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        fs::create_dir_all(&path)?;
        let mut kv_log = KvLog::open(latest_log_path(&path)?)?;
        kv_log.load()?;

        Ok(Self { path, kv_log })
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
        self.kv_log.set(key, value)?;
        if self.kv_log.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }
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
        self.kv_log.get(key)
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
        self.kv_log.remove(key)
    }

    /// Save space by clearing stale entries in the log.
    fn compact(&mut self) -> Result<()> {
        // The new log file for merged entries
        let tmp_log_path = self.path.join("kvs.log.new");
        let mut new_writer = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&tmp_log_path)?,
        );

        // Compact the log by key order.
        // Mostly read sequentially; with a sorted index like a b-tree,
        // there would be no copying of the index.
        let mut new_pos = 0; // pos in the new log file
        let mut new_index = BTreeMap::new(); // index map for the new log file
        for (key, cmd_pos) in &self.kv_log.index {
            if self.kv_log.reader.pos != cmd_pos.pos {
                self.kv_log.reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            }

            let mut entry_reader = (&mut self.kv_log.reader).take(cmd_pos.len);
            let len = io::copy(&mut entry_reader, &mut new_writer)?;
            new_index.insert(key.clone(), (new_pos..new_pos + len).into());
            new_pos += len;
        }
        // Explicit flush and close before dropping the writer. We would not rely the destructor
        // to do it, particularly in a case where data must not be lost.
        new_writer.flush()?;

        drop(new_writer);

        // As all entries are written to the log, we can safely rename it to a valid log file name
        let log_path = new_log_path(&self.path);
        fs::rename(tmp_log_path, &log_path)?;

        // Reopen using the new file name
        let mut kv_log = KvLog::open(&log_path)?;
        // Use the index map built on writing instead of reloading the log file
        kv_log.index = new_index;
        // Update the KvLog we are using
        mem::swap(&mut self.kv_log, &mut kv_log);

        // Close old log file before removing it. (It's a must on Windows I think)
        let old_path = kv_log.path.clone();
        // The old file is useless. It's safe we just drop it.
        drop(kv_log);
        fs::remove_file(old_path)?;

        Ok(())
    }
}

struct KvLog {
    path: PathBuf,
    /// Writer of the log
    writer: BufWriterWithPos<File>,
    /// Reader of the log
    reader: BufReaderWithPos<File>,
    /// Stores keys and the pos of the last command
    index: BTreeMap<String, CommandPos>,
    uncompacted: u64,
}

impl KvLog {
    // Pay attention that it does not load the log file automatically
    fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let mut writer =
            BufWriterWithPos::new(OpenOptions::new().create(true).append(true).open(&path)?)?;
        // Because file mode is set to append, we need to set pos to end of file manually to keep synced
        writer.seek(SeekFrom::End(0))?;

        let reader = BufReaderWithPos::new(File::open(&path)?)?;

        Ok(Self {
            path,
            reader,
            writer,
            index: BTreeMap::new(),
            uncompacted: 0,
        })
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let command = Command::new(CommandType::Set, key.to_owned(), value.to_owned());
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, &command)?;
        self.writer.flush()?;
        if let Some(old_cmd) = self.index.insert(key, (pos..self.writer.pos).into()) {
            self.uncompacted += old_cmd.len;
        }
        Ok(())
    }

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

    pub fn remove(&mut self, key: String) -> Result<()> {
        let command = Command::new(CommandType::Remove, key.to_owned(), "".to_string());
        serde_json::to_writer(&mut self.writer, &command)?;
        self.writer.flush()?;

        self.index.remove(&key);

        Ok(())
    }

    /// Load from the log file.
    fn load(&mut self) -> Result<()> {
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
                    if let Some(old_cmd) = self.index.insert(key, (pos..new_pos).into()) {
                        self.uncompacted += old_cmd.len;
                    }
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

// Log files are named after milliseconds since epoch with a "log" extension name.
// This function finds the latest log file.
fn latest_log_path(dir: impl AsRef<Path>) -> Result<PathBuf> {
    let mut latest: Option<PathBuf> = None;
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_file() && path.extension() == Some("log".as_ref()) {
            latest = cmp::max(latest, Some(path));
        }
    }

    if let Some(path) = latest {
        Ok(path)
    } else {
        Ok(new_log_path(dir))
    }
}

fn new_log_path(dir: impl AsRef<Path>) -> PathBuf {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before epoch");
    dir.as_ref().join(format!("{}.log", time.as_millis()))
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
