use ignore::WalkBuilder;
use ngram::split_ngrams;
use puffin_query::QueryNode;
use sstable::SSTable;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::{self, BufWriter};
use std::iter::Peekable;
use std::vec;
use std::{fs, io::Read};

mod ngram;

const MAX_SIZE: usize = 2 << 20;

pub mod search {
    include!(concat!(env!("OUT_DIR"), "/search.rs"));
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FileId(u64);

#[derive(Clone, Default)]
struct FileIds(Vec<FileId>);

impl FileIds {
    fn insert(&mut self, other: FileId) {
        self.0.push(other);
    }
}

impl From<Vec<u8>> for FileIds {
    fn from(value: Vec<u8>) -> Self {
        Self(
            value
                .chunks(8)
                .map(|chunk| {
                    let mut bytes = [0; 8];
                    bytes.copy_from_slice(chunk);
                    FileId(u64::from_le_bytes(bytes))
                })
                .collect::<Vec<_>>(),
        )
    }
}

impl From<FileIds> for Vec<u8> {
    fn from(val: FileIds) -> Self {
        val.0.iter().flat_map(|v| u64::to_le_bytes(v.0)).collect()
    }
}

pub struct Metadata<K, V> {
    data: BTreeMap<K, V>,
}

impl<K, V> Metadata<K, V>
where
    V: prost::Message,
{
    pub fn flush(&mut self) -> Result<(), io::Error> {
        let name = "metadata";
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(format!("database/{}", name))
            .unwrap();

        let mut data_writer = BufWriter::new(file);

        for (k, v) in self.data.iter() {
            // data_writer.write(k);
        }

        Ok(())
    }
}

pub struct Index {
    content_ngrams: SSTable<FileIds>,
    file_meta: BTreeMap<FileId, Vec<search::File>>,
}

impl Index {
    pub fn new(loc: &str) -> Self {
        Index {
            content_ngrams: SSTable::new(loc, 100000),
            file_meta: BTreeMap::new(),
        }
    }

    pub fn flush(&mut self) -> Result<(), io::Error> {
        self.content_ngrams.flush()?;
        Ok(())
    }

    pub fn search(&self, query: QueryNode) -> Vec<search::File> {
        let mut results: Vec<FileId> = Vec::new();
        let keys = self.file_meta.keys().cloned().collect();
        let mut iters = Box::new(And::new(vec![
            Box::new(All::new(keys)),
            self.match_iter(query),
        ]));
        while let Some(fid) = iters.next() {
            results.push(fid);
        }

        results
            .iter()
            .flat_map(|f| self.file_meta.get(f).cloned().unwrap_or_default())
            .collect()
    }

    pub fn index(&mut self, dir_path: &str) {
        let walker = WalkBuilder::new(dir_path).standard_filters(true).build();
        let mut contents = String::new();

        for entry in walker {
            let entry = entry.expect("Unable to get directory entry");
            let path = entry.path();

            log::info!("indexing {:?}", path);

            if path.is_file() {
                let mut file = fs::File::open(path).expect("Unable to open file");

                contents.clear();
                match file.read_to_string(&mut contents) {
                    Ok(size) => {
                        if size > MAX_SIZE {
                            println!("skipping {:?}, too large", path);
                            continue;
                        }
                    }
                    Err(_) => continue,
                };

                let file_name = path.to_str().unwrap().to_string();
                let file_id = hash_filename(path.to_str().unwrap());

                let files_vec = self.file_meta.entry(file_id.clone()).or_default();
                files_vec.push(search::File {
                    filename: file_name,
                    content: contents.clone(),
                    file_type: "unknown".into(),
                });

                self.collect_trigrams(&file_id, &contents);
            }
        }
    }

    pub fn collect_trigrams(&mut self, file_id: &FileId, src: &str) {
        log::info!("collecting trigrams");
        for line in src.lines() {
            for (trigram, _) in split_ngrams(line) {
                let mut current = self
                    .content_ngrams
                    .get(trigram.to_string().as_str())
                    .unwrap_or_default();
                current.insert(file_id.clone());
                self.content_ngrams
                    .insert(trigram.to_string().as_str(), current)
                    .unwrap();
            }
        }
        log::info!("collecting trigrams done");
    }

    fn match_iter(&self, query: QueryNode) -> Box<dyn MatchIter> {
        match query {
            QueryNode::Or { lhs, rhs } => {
                Box::new(Or::new(vec![self.match_iter(*lhs), self.match_iter(*rhs)]))
            }
            QueryNode::And { lhs, rhs } => {
                Box::new(And::new(vec![self.match_iter(*lhs), self.match_iter(*rhs)]))
            }
            QueryNode::Not(q) => Box::new(Not::new(self.match_iter(*q))),
            QueryNode::Lang(_) => todo!(),
            QueryNode::File(_) => todo!(),
            QueryNode::Term(t) => Box::new(ContentGrams::new(t, self)),
            QueryNode::Regex(_) => todo!(),
        }
    }
}

fn hash_filename(filename: &str) -> FileId {
    let mut s = DefaultHasher::new();
    filename.hash(&mut s);
    FileId(s.finish())
}

trait MatchIter {
    fn matches(&self, fid: &FileId) -> bool;
    fn next(&mut self) -> Option<FileId>;
}

struct All {
    iter: Peekable<vec::IntoIter<FileId>>,
}

impl All {
    pub fn new(iter: Vec<FileId>) -> Self {
        Self {
            iter: iter.into_iter().peekable(),
        }
    }
}

impl MatchIter for All {
    fn matches(&self, _: &FileId) -> bool {
        true
    }

    fn next(&mut self) -> Option<FileId> {
        self.iter.next()
    }
}

struct Not(Box<dyn MatchIter>);

impl Not {
    fn new(q: Box<dyn MatchIter>) -> Self {
        Not(q)
    }
}

impl MatchIter for Not {
    fn matches(&self, fid: &FileId) -> bool {
        !self.0.matches(fid)
    }

    fn next(&mut self) -> Option<FileId> {
        None
    }
}

struct Or {
    iterators: Vec<Box<dyn MatchIter>>,
}

impl Or {
    pub fn new(iterators: Vec<Box<dyn MatchIter>>) -> Self {
        Self { iterators }
    }
}

impl MatchIter for Or {
    fn matches(&self, fid: &FileId) -> bool {
        self.iterators.iter().any(|i| i.matches(fid))
    }

    fn next(&mut self) -> Option<FileId> {
        'outer: loop {
            let current = self.iterators.first_mut().and_then(|i| i.next());
            current.as_ref()?;
            let current = current.unwrap();
            for iterator in self.iterators.iter_mut().skip(1) {
                if !iterator.matches(&current) {
                    continue 'outer;
                }
            }
            return Some(current);
        }
    }
}

struct And {
    current: Option<FileId>,
    iterators: Vec<Box<dyn MatchIter>>,
}

impl And {
    pub fn new(iterators: Vec<Box<dyn MatchIter>>) -> Self {
        let mut x = Self {
            current: None,
            iterators,
        };
        x.current = x.advance();
        x
    }
}

impl And {
    fn advance(&mut self) -> Option<FileId> {
        'outer: loop {
            let current = self.iterators.first_mut().and_then(|i| i.next());
            current.as_ref()?;
            let current = current.unwrap();
            for iterator in self.iterators.iter_mut().skip(1) {
                if !iterator.matches(&current) {
                    continue 'outer;
                }
            }
            return Some(current);
        }
    }
}

impl MatchIter for And {
    fn matches(&self, fid: &FileId) -> bool {
        self.iterators.iter().all(|i| i.matches(fid))
    }

    fn next(&mut self) -> Option<FileId> {
        let current = self.current.take();
        self.current = self.advance();
        current
    }
}

struct ContentGrams(Vec<FileId>);

impl ContentGrams {
    pub fn new(q: String, index: &Index) -> Self {
        let mut matching_file_ids: BTreeSet<FileId> = BTreeSet::new();
        for (trigram, _) in split_ngrams(&q) {
            let mut set: BTreeSet<FileId> = BTreeSet::new();

            if let Some(files) = index.content_ngrams.get(trigram.to_string().as_str()) {
                set.clear();
                set.extend(files.0.clone());
                if matching_file_ids.is_empty() {
                    matching_file_ids = set;
                } else {
                    matching_file_ids = matching_file_ids.intersection(&set).cloned().collect();
                }
            }
        }

        Self(matching_file_ids.into_iter().rev().collect())
    }
}

impl MatchIter for ContentGrams {
    fn matches(&self, fid: &FileId) -> bool {
        self.0.contains(fid)
    }

    fn next(&mut self) -> Option<FileId> {
        self.0.pop()
    }
}
