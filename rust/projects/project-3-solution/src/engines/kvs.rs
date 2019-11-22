use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use super::KvsEngine;
use crate::{KvsError, Result};

const COMPACTION_THRESHOLD: u64 = 1024;

/// The `KvStore` stores string key/value pairs.
///
/// Key/value pairs are stored in memory and also persisted to disk in a log.
/// Log files are named after monotonically increasing generation numbers with
/// a `log` extension name. Index as a `BTreeMap` in memory stores the keys and
/// the value positions for fast query.
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
    /// Writer of the current log
    writer: BufWriterWithPos<File>,
    /// Map generation number to the file reader
    readers: HashMap<u64, BufReaderWithPos<File>>,
    /// Stores keys and the pos of the last command
    index: BTreeMap<String, CommandPos>,
    /// The number of bytes representing "stale" commands
    /// that could be deleted during a compaction.
    uncompacted: u64,
    /// Current generation number
    current_gen: u64,
}

impl KvStore {
    /// Opens the store with the given path.
    ///
    /// # Errors
    ///
    /// It propagates I/O or deserialization errors during the log replay.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        fs::create_dir_all(&path)?;

        // A list of log file names. The file names looks like a sequence of generated numbers.
        let gen_list = sorted_gen_list(&path)?;
        let mut uncompacted = 0;

        // Initialized index and log readers.
        let mut index = BTreeMap::new();
        let mut readers = HashMap::new(); // one reader for one log file

        // Loop over multiple log files if any in a directory
        for &gen in &gen_list {
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, gen))?)?;
            uncompacted += load(gen, &mut reader, &mut index)?;
            readers.insert(gen, reader);
        }

        // Increment log file name from the last generated number and create new log file with it.
        let current_gen = gen_list.last().unwrap_or(&0) + 1;
        let writer = new_log_file(&path, current_gen, &mut readers)?;

        Ok(Self {
            path,
            readers,
            writer,
            index,
            uncompacted,
            current_gen,
        })
    }

    /// Save space by clearing stale entries in the log.
    fn compact(&mut self) -> Result<()> {
        // Increase current gen number by 2. current_gen + 1 is for the compaction file.
        let compaction_gen = self.current_gen + 1;
        self.current_gen += 2;

        self.writer = self.new_log_file(self.current_gen)?;

        let mut compaction_writer = self.new_log_file(compaction_gen)?;

        // Compact the log by key order.
        // Mostly read sequentially; with a sorted index like a b-tree,
        // there would be no copying of the index.
        let mut new_pos = 0; // pos in the new log file
        for cmd_pos in &mut self.index.values_mut() {
            let reader = self
                .readers
                .get_mut(&cmd_pos.gen)
                .expect("Cannot find log reader");
            if reader.pos != cmd_pos.pos {
                reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            }

            let mut entry_reader = reader.take(cmd_pos.len);
            let len = io::copy(&mut entry_reader, &mut compaction_writer)?;
            *cmd_pos = (compaction_gen, new_pos..new_pos + len).into();
            new_pos += len;
        }

        // Explicit flush and close before dropping the writer. We would not rely the destructor
        // to do it, particularly in a case where data must not be lost.
        compaction_writer.flush()?;

        // Remove stale log files
        let stale_gens: Vec<_> = self
            .readers
            .keys()
            .filter(|&&gen| gen < compaction_gen)
            .cloned()
            .collect();
        for stale_gen in stale_gens {
            self.readers.remove(&stale_gen);
            fs::remove_file(log_path(&self.path, stale_gen))?;
        }

        // Reset uncompacted after compaction
        self.uncompacted = 0;

        Ok(())
    }

    /// Create a new log file with given generation number and add the reader to the readers map.
    ///
    /// Returns the writer to the log.
    fn new_log_file(&mut self, gen: u64) -> Result<BufWriterWithPos<File>> {
        new_log_file(&self.path, gen, &mut self.readers)
    }
}

impl KvsEngine for KvStore {
    /// Set a given key and value Strings in the store.
    ///
    /// If the key already exists, the previous value will be overwritten.
    ///
    /// # Errors
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
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let command = Command::set(key, value);
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, &command)?;
        self.writer.flush()?;
        if let Command::Set { key, .. } = command {
            // Storing log pointers in the index. Log pointers is of type CommandPos.
            if let Some(old_cmd) = self
                .index
                .insert(key, (self.current_gen, pos..self.writer.pos).into())
            {
                self.uncompacted += old_cmd.len;
            }
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
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
    fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.index.get(&key) {
            let reader = self
                .readers
                .get_mut(&cmd_pos.gen)
                .expect("Cannot find log reader");
            reader.seek(SeekFrom::Start(cmd_pos.pos))?;

            let cmd_reader = reader.take(cmd_pos.len);
            if let Command::Set { value, .. } = serde_json::from_reader(cmd_reader)? {
                Ok(Some(value))
            } else {
                Err(KvsError::UnexpectedCommandType)
            }
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
    fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let command = Command::remove(key);
            serde_json::to_writer(&mut self.writer, &command)?;
            self.writer.flush()?;

            if let Command::Remove { key } = command {
                let old_cmd = self.index.remove(&key).expect("key not found");
                self.uncompacted += old_cmd.len;
            }

            Ok(())
        } else {
            Err(KvsError::KeyNotFound)
        }
    }
}

/// Enum representing a command
#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Set { key: String, value: String },
    Remove { key: String },
}

impl Command {
    fn set(key: String, value: String) -> Command {
        Command::Set { key, value }
    }

    fn remove(key: String) -> Command {
        Command::Remove { key }
    }
}

/// Represents the JSON-serialized command in the log.
struct CommandPos {
    /// Log files are named after a generation number.
    /// `gen` gives us the log filename the command was stored.
    gen: u64,
    /// Position.
    pos: u64,
    /// Length.
    len: u64,
}

impl From<(u64, Range<u64>)> for CommandPos {
    fn from((gen, range): (u64, Range<u64>)) -> Self {
        Self {
            gen,
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
        let len = self.reader.read(buf)?;
        self.pos += len as u64;

        Ok(len)
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

/// Log files are named after a generation number with a "log" extension name.
///
/// Returns sorted generation numbers in the given directory
fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gen_list: Vec<u64> = fs::read_dir(&path)?
        .flat_map(|res| -> Result<_> { Ok(res?.path()) })
        .filter(|path| path.is_file() && path.extension() == Some("log".as_ref()))
        .flat_map(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.trim_end_matches(".log"))
                .map(str::parse::<u64>)
        })
        .flatten()
        .collect();

    gen_list.sort_unstable();
    Ok(gen_list)
}

fn log_path(dir: &Path, gen: u64) -> PathBuf {
    dir.join(format!("{}.log", gen))
}

/// Create a new log file with given generation number and add the reader to the readers map.
///
/// Returns the writer to the log.
fn new_log_file(
    path: &Path,
    gen: u64,
    readers: &mut HashMap<u64, BufReaderWithPos<File>>,
) -> Result<BufWriterWithPos<File>> {
    let path = log_path(&path, gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?,
    )?;
    readers.insert(gen, BufReaderWithPos::new(File::open(&path)?)?);
    Ok(writer)
}

/// Load the whole log file and store value positions in the index map.
///
/// Returns `uncompacted`, which is number of bytes that can be saved after a compaction.
fn load(
    gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &mut BTreeMap<String, CommandPos>,
) -> Result<u64> {
    let mut uncompacted = 0;

    // To make sure we read from the beginning of the file.
    let mut pos = reader.seek(SeekFrom::Start(0))?;
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();

    while let Some(cmd) = stream.next() {
        let new_pos = stream.byte_offset() as u64;
        match cmd? {
            Command::Set { key, .. } => {
                if let Some(old_cmd) = index.insert(key, (gen, pos..new_pos).into()) {
                    uncompacted += old_cmd.len;
                }
            }
            Command::Remove { key } => {
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.len;
                }

                // The "remove" command itself can be deleted in the next compaction so we add
                // its length to `uncompacted`.
                uncompacted += new_pos - pos;
            }
        }

        pos = new_pos;
    }

    Ok(uncompacted)
}
