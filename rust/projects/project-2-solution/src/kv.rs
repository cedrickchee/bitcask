use std::collections::HashMap;
use std::io::Result;
use std::path::Path;

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
    storage: HashMap<String, String>,
}

impl KvStore {
    /// Creates a new `KvStore`.
    ///
    /// # Example
    ///
    /// ```
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::new();
    /// ```
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
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
        self.storage.insert(key, value);
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
        Ok(self.storage.get(&key).cloned())
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
    pub fn remove(&mut self, key: String) -> Result<()> {
        self.storage.remove(&key);
        Ok(())
    }

    /// Open is ...
    pub fn open(_path: &Path) -> Result<Self> {
        unimplemented!();
    }
}
