use std::io;
mod disktable;
mod memtable;
mod rich_file;

pub struct SSTable<V> {
    memtable: memtable::Memtable<V>,
    disktable: disktable::Disktable<V>,
}

impl<V> SSTable<V>
where
    V: Clone + From<Vec<u8>> + Into<Vec<u8>>,
{
    pub fn new(dir_name: &str, mem_max_entry: usize) -> SSTable<V> {
        std::fs::create_dir_all(dir_name)
            .unwrap_or_else(|_| panic!("failed to create directory {}", dir_name));
        SSTable {
            memtable: memtable::Memtable::new(mem_max_entry),
            disktable: disktable::Disktable::new(dir_name).unwrap(),
        }
    }

    pub fn get(&self, key: &str) -> Option<V> {
        self.memtable.get(key).cloned()
        // match self.memtable.get(key) {
        //     Some(value) => Some(value.clone()),
        //     None => self.disktable.find(key),
        // }
    }

    pub fn insert(&mut self, key: &str, value: V) -> Result<(), io::Error> {
        self.memtable
            .set(key, value)
            .on_flush(|mem| self.disktable.flush(mem))
    }

    pub fn delete(&mut self, key: &str) {
        self.memtable.delete(key);
    }

    pub fn clear(&mut self) -> Result<(), io::Error> {
        self.disktable.clear()?;
        self.memtable.clear();
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), io::Error> {
        self.disktable.flush(self.memtable.flush())
    }
}

#[cfg(test)]
mod tests {
    use crate::SSTable;
    #[test]
    fn test_sstable() {
        let key = |i| format!("key-{}", i);
        let value = |i| format!("value-{}", i).into_bytes();

        let mut sst = SSTable::new("./test_tmp", 200);
        assert!(sst.clear().is_ok());
        // get -> set -> get
        (1..300).for_each(|i| {
            assert_eq!(sst.get(&key(i)), None);
            sst.insert(&key(i), value(i)).expect("success");
            assert_eq!(sst.get(&key(i)), Some(value(i)));
        });
        // get -> delete -> get
        (1..300).for_each(|i| {
            assert_eq!(sst.get(&key(i)), Some(value(i)));
            sst.delete(&key(i));
            assert_eq!(sst.get(&key(i)), None);
        });
        // get
        (1..300).for_each(|i| {
            assert_eq!(sst.get(&key(i)), None);
        });
    }

    #[test]
    fn test_sstabl_tombstones() {
        let key = |i| format!("key-{}", i);
        let value = |i| format!("value-{}", i).into_bytes();
        let mut sst = SSTable::new("./test_tmp2", 3);
        assert!(sst.clear().is_ok());
        (1..=5).for_each(|i| {
            sst.insert(&key(i), value(i)).expect("success");
        });
        sst.delete(&key(2));
        // restore WAL
        // memtable: [4, 5], tombstone: [2], disktable: [1, 2, 3]
        let sst = SSTable::new("./test_tmp2", 3);
        assert_eq!(sst.get(&key(1)), Some(value(1)));
        assert_eq!(sst.get(&key(2)), None);
        assert_eq!(sst.get(&key(3)), Some(value(3)));
        assert_eq!(sst.get(&key(4)), Some(value(4)));
        assert_eq!(sst.get(&key(5)), Some(value(5)));
    }
}
