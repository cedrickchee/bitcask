use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_skiplist::SkipMap;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

use super::KvsEngine;
use crate::{KvsError, Result};

const COMPACTION_THRESHOLD: u64 = 1024;

/// The `KvStore` stores string key/value pairs.
///
/// Key/value pairs are stored in memory and also persisted to disk in a log.
/// Log files are named after monotonically increasing generation numbers with
/// a `log` extension name. Index as a skip list in memory stores the keys and
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
#[derive(Clone)]
pub struct KvStore {
    /// Directory for the log and other data
    path: Arc<PathBuf>,
    /// The log reader
    reader: KvStoreReader,
    /// The in-memory index from key to log pointer
    index: Arc<SkipMap<String, CommandPos>>,
    /// The log writer
    writer: Arc<Mutex<KvStoreWriter>>,
}

impl KvStore {
    /// Opens the store with the given path.
    ///
    /// This will create a new directory if the given one does not exist.
    ///
    /// # Errors
    ///
    /// It propagates I/O or deserialization errors during the log replay.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = Arc::new(path.into());
        fs::create_dir_all(&*path)?;

        // A list of log file names. The file names looks like a sequence of generated numbers.
        let gen_list = sorted_gen_list(&path)?;
        let mut uncompacted = 0;

        // Initialized index and log readers.
        let index = Arc::new(SkipMap::new());
        let mut readers = BTreeMap::new(); // one reader for one log file

        // Loop over multiple log files if any in a directory
        for &gen in &gen_list {
            let mut reader = BufReaderWithPos::new(File::open(log_path(&path, gen))?)?;
            uncompacted += load(gen, &mut reader, &*index)?;
            readers.insert(gen, reader);
        }

        // Increment log file name from the last generated number and create new log file with it.
        let current_gen = gen_list.last().unwrap_or(&0) + 1;
        let writer = new_log_file(&path, current_gen)?;

        let reader = KvStoreReader {
            path: Arc::clone(&path),
            readers: RefCell::new(BTreeMap::new()),
            safe_point: Arc::new(AtomicU64::new(0)),
        };

        let writer = KvStoreWriter {
            path: Arc::clone(&path),
            writer,
            reader: reader.clone(),
            uncompacted,
            current_gen,
            index: Arc::clone(&index),
        };

        Ok(Self {
            path,
            reader,
            index,
            writer: Arc::new(Mutex::new(writer)),
        })
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
    fn set(&self, key: String, value: String) -> Result<()> {
        self.writer.lock().unwrap().set(key, value)
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
    fn get(&self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.index.get(&key) {
            if let Command::Set { value, .. } = self.reader.read_command(*cmd_pos.value())? {
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
    fn remove(&self, key: String) -> Result<()> {
        self.writer.lock().unwrap().remove(key)
    }
}

/// A single thread reader.
///
/// Each `KvStore` instance has its own `KvStoreReader` and `KvStoreReader`s open the same files
/// separately. So the user can read concurrently through multiple `KvStore`s in different threads.
struct KvStoreReader {
    path: Arc<PathBuf>,
    // Map generation number to the file reader
    readers: RefCell<BTreeMap<u64, BufReaderWithPos<File>>>,
    // Generation of the latest compaction file.
    // Readers with a generation before safe_point can be closed.
    safe_point: Arc<AtomicU64>,
}

impl Clone for KvStoreReader {
    fn clone(&self) -> Self {
        Self {
            path: Arc::clone(&self.path),
            // Don't use other KvStoreReader's readers
            readers: RefCell::new(BTreeMap::new()),
            safe_point: Arc::clone(&self.safe_point),
        }
    }
}

impl KvStoreReader {
    /// Read the log file at the given `CommandPos` and deserialize it to `Command`.
    fn read_command(&self, cmd_pos: CommandPos) -> Result<Command> {
        self.build_cmd_reader(cmd_pos, |cmd_reader| {
            Ok(serde_json::from_reader(cmd_reader)?)
        })
    }

    /// Build command reader from reader and `CommandPos`.
    fn build_cmd_reader<F, R>(&self, cmd_pos: CommandPos, f: F) -> Result<R>
    where
        F: FnOnce(io::Take<&mut BufReaderWithPos<File>>) -> Result<R>,
    {
        self.close_stale_handles();

        let mut readers = self.readers.borrow_mut();

        // Open the file if we haven't opened it in this `KvStoreReader`.
        // We don't use entry API here because we want the errors to be propogated.
        if !readers.contains_key(&cmd_pos.gen) {
            let reader = BufReaderWithPos::new(File::open(log_path(&self.path, cmd_pos.gen))?)?;
            readers.insert(cmd_pos.gen, reader);
        }

        let reader = readers
            .get_mut(&cmd_pos.gen)
            .expect("Cannot find log reader");
        reader.seek(SeekFrom::Start(cmd_pos.pos))?;

        let cmd_reader = reader.take(cmd_pos.len);
        f(cmd_reader)
    }

    /// Close file handles with generation number less than safe_point.
    ///
    /// `safe_point` is updated to the latest compaction gen after a compaction finishes.
    /// The compaction generation contains the sum of all operations before it and the
    /// in-memory index contains no entries with generation number less than safe_point.
    /// So we can safely close those file handles and the stale files can be deleted.
    fn close_stale_handles(&self) {
        let mut readers = self.readers.borrow_mut();

        while !readers.is_empty() {
            let first_gen = *readers.keys().next().unwrap();
            if self.safe_point.load(Ordering::SeqCst) <= first_gen {
                break;
            }
            readers.remove(&first_gen);
        }
    }
}

struct KvStoreWriter {
    path: Arc<PathBuf>,
    writer: BufWriterWithPos<File>,
    reader: KvStoreReader,
    /// The number of bytes representing "stale" commands
    /// that could be deleted during a compaction.
    uncompacted: u64,
    /// Current generation number
    current_gen: u64,
    index: Arc<SkipMap<String, CommandPos>>,
}

impl KvStoreWriter {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        let command = Command::set(key, value);
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, &command)?;
        self.writer.flush()?;
        if let Command::Set { key, .. } = command {
            // Storing log pointers in the index. Log pointers is of type CommandPos.
            if let Some(old_cmd) = self.index.get(&key) {
                self.uncompacted += old_cmd.value().len;
            }
            self.index
                .insert(key, (self.current_gen, pos..self.writer.pos).into());
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }

        Ok(())
    }

    fn remove(&mut self, key: String) -> Result<()> {
        if self.index.contains_key(&key) {
            let command = Command::remove(key);
            let pos = self.writer.pos;
            serde_json::to_writer(&mut self.writer, &command)?;
            self.writer.flush()?;

            if let Command::Remove { key } = command {
                let old_cmd = self.index.remove(&key).expect("key not found");
                self.uncompacted += old_cmd.value().len;

                // The "remove" command itself can be deleted in the next compaction
                // so we add its length to `uncompacted`.
                self.uncompacted += self.writer.pos - pos;
            }

            if self.uncompacted > COMPACTION_THRESHOLD {
                self.compact()?;
            }

            Ok(())
        } else {
            Err(KvsError::KeyNotFound)
        }
    }

    /// Save space by clearing stale entries in the log.
    fn compact(&mut self) -> Result<()> {
        // Increase current gen number by 2. current_gen + 1 is for the compaction file.
        let compaction_gen = self.current_gen + 1;
        self.current_gen += 2;

        self.writer = new_log_file(&self.path, self.current_gen)?;

        let mut compaction_writer = new_log_file(&self.path, compaction_gen)?;

        // Compact the log by key order.
        // Mostly read sequentially; with a sorted index like a b-tree,
        // there would be no copying of the index.
        let mut new_pos = 0; // pos in the new log file
        for entry in &mut self.index.iter() {
            let len = self
                .reader
                .build_cmd_reader(*entry.value(), |mut entry_reader| {
                    Ok(io::copy(&mut entry_reader, &mut compaction_writer)?)
                })?;
            self.index.insert(
                entry.key().clone(),
                (compaction_gen, new_pos..new_pos + len).into(),
            );
            new_pos += len;
        }

        // Explicit flush and close before dropping the writer. We would not rely the destructor
        // to do it, particularly in a case where data must not be lost.
        compaction_writer.flush()?;

        self.reader
            .safe_point
            .store(compaction_gen, Ordering::SeqCst);
        self.reader.close_stale_handles();

        // Remove stale log files.
        //
        // Note that actually these files are not deleted immediately because `KvStoreReader`s
        // still keep open file handles. When `KvStoreReader` is used next time, it will clear
        // its stale file handles. On Unix, the files will be deleted after all the handles
        // are closed. On Windows, the deletions below will fail and stale files are expected
        // to be deleted in the next compaction.
        let stale_gens = sorted_gen_list(&self.path)?
            .into_iter()
            .filter(|&gen| gen < compaction_gen);
        for stale_gen in stale_gens {
            let file_path = log_path(&self.path, stale_gen);
            if let Err(e) = fs::remove_file(&file_path) {
                error!("{:?} cannot be deleted: {}", file_path, e);
            }
        }

        // Reset uncompacted after compaction
        self.uncompacted = 0;

        Ok(())
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
#[derive(Copy, Clone)]
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

/// Create a new log file with given generation number.
///
/// Returns the writer to the log.
fn new_log_file(path: &Path, gen: u64) -> Result<BufWriterWithPos<File>> {
    let path = log_path(&path, gen);
    let writer = BufWriterWithPos::new(
        OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&path)?,
    )?;
    Ok(writer)
}

/// Load the whole log file and store value positions in the index map.
///
/// Returns `uncompacted`, which is number of bytes that can be saved after a compaction.
fn load(
    gen: u64,
    reader: &mut BufReaderWithPos<File>,
    index: &SkipMap<String, CommandPos>,
) -> Result<u64> {
    let mut uncompacted = 0;

    // To make sure we read from the beginning of the file.
    let mut pos = reader.seek(SeekFrom::Start(0))?;
    let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();

    while let Some(cmd) = stream.next() {
        let new_pos = stream.byte_offset() as u64;
        match cmd? {
            Command::Set { key, .. } => {
                if let Some(old_cmd) = index.get(&key) {
                    uncompacted += old_cmd.value().len;
                }
                index.insert(key, (gen, pos..new_pos).into());
            }
            Command::Remove { key } => {
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.value().len;
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
