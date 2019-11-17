use std::collections::HashMap;

/// A key-value store of String keys and values.
pub struct KvStore {
    storage: HashMap<String, String>,
}

impl KvStore {
    /// Constructs a new `KvStore`.
    ///
    /// # Examples
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
    /// # Examples
    ///
    /// ```
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::new();
    /// store.set(String::from("my_key"), String::from("my_value"));
    /// ```
    pub fn set(&mut self, key: String, value: String) {
        self.storage.insert(key, value);
    }

    /// Get a value from the store using a key String.
    ///
    /// # Examples
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
    pub fn get(&self, key: String) -> Option<String> {
        match self.storage.get(&key) {
            Some(value) => Some(value.clone()),
            None => None,
        }
    }

    /// Remove a given key from the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use kvs::KvStore;
    ///
    /// let mut store = KvStore::new();
    /// store.remove(String::from("my_key"));
    /// ```
    pub fn remove(&mut self, key: String) {
        self.storage.remove(&key);
    }
}
