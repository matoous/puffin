use std::{collections::BTreeMap, io};

pub(crate) struct MemtableEntries<V> {
    pub entries: BTreeMap<String, V>,
}

impl<V> MemtableEntries<V> {
    pub fn get(&self, key: &str) -> Option<&V> {
        self.entries.get(key)
    }
}

pub(crate) struct MemtableOnFlush<V> {
    flushed: Option<MemtableEntries<V>>,
}

impl<V> MemtableOnFlush<V> {
    pub fn on_flush(self, f: impl FnOnce(MemtableEntries<V>) -> io::Result<()>) -> io::Result<()> {
        match self.flushed {
            Some(flushed) => f(flushed),
            None => Ok(()),
        }
    }
}

pub struct Memtable<V> {
    max_entry: usize,
    underlying: BTreeMap<String, V>,
}

impl<V> Memtable<V> {
    pub fn new(max_entry: usize) -> Memtable<V> {
        Memtable {
            max_entry,
            underlying: BTreeMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&V> {
        self.underlying.get(key)
    }

    pub fn set(&mut self, key: &str, value: V) -> MemtableOnFlush<V> {
        self.underlying.insert(key.into(), value);
        if self.underlying.len() > self.max_entry {
            log::trace!("flush!");
            MemtableOnFlush {
                flushed: Some(self.flush()),
            }
        } else {
            MemtableOnFlush { flushed: None }
        }
    }

    pub fn delete(&mut self, key: &str) {
        self.underlying.remove(key);
    }

    pub fn clear(&mut self) {
        self.underlying.clear();
    }

    pub fn flush(&mut self) -> MemtableEntries<V> {
        let contents = std::mem::take(&mut self.underlying);
        MemtableEntries { entries: contents }
    }
}
