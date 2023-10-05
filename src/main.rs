use ignore::WalkBuilder;
use ngram::{split_ngrams, Ngram};
use std::collections::BTreeMap;
use std::collections::{hash_map::DefaultHasher, HashSet};
use std::hash::{Hash, Hasher};
use std::{fs, io::Read};

mod ngram;

pub mod search {
    include!(concat!(env!("OUT_DIR"), "/search.rs"));
}

const MAX_SIZE: usize = 2 << 20;

fn main() {
    let dir_path = "./";

    let mut index = Index::new();
    index.index(dir_path);

    let query = "RUNE";
    let result = index.search(query.into());

    for f in result {
        println!("match in: {:?}", f.filename);
    }
}

struct Index {
    // maps trigram to (file id, offset in file)
    content_ngrams: BTreeMap<Ngram, Vec<(u64, u64)>>,
    file_meta: BTreeMap<u64, Vec<search::File>>,
}

impl Index {
    pub fn new() -> Self {
        Index {
            content_ngrams: BTreeMap::new(),
            file_meta: BTreeMap::new(),
        }
    }

    pub fn search(&self, query: String) -> Vec<search::File> {
        let mut matching_file_ids: HashSet<u64> = HashSet::new();

        for (trigram, _) in split_ngrams(&query) {
            let mut set: HashSet<u64> = HashSet::new();

            if let Some(files) = self.content_ngrams.get(&trigram) {
                set.clear();
                set.extend(files.iter().map(|(file_id, _)| file_id).cloned());
                if matching_file_ids.is_empty() {
                    matching_file_ids = set;
                } else {
                    matching_file_ids = matching_file_ids.intersection(&set).cloned().collect();
                }
            }
        }

        matching_file_ids
            .iter()
            .map(|f| self.file_meta.get(f).cloned().unwrap_or_default())
            .flatten()
            .collect()
    }

    pub fn index(&mut self, dir_path: &str) {
        let walker = WalkBuilder::new(dir_path).standard_filters(true).build();
        let mut contents = String::new();

        for entry in walker {
            let entry = entry.expect("Unable to get directory entry");
            let path = entry.path();

            if path.is_file() {
                let mut file = fs::File::open(&path).expect("Unable to open file");

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

                let files_vec = self.file_meta.entry(file_id).or_insert(Vec::new());
                files_vec.push(search::File {
                    filename: file_name,
                    content: contents.clone(),
                    file_type: "unknown".into(),
                });

                self.collect_trigrams(file_id, &contents);
            }
        }
    }

    fn collect_trigrams(&mut self, file_id: u64, src: &str) {
        for line in src.lines() {
            for (trigram, offset) in split_ngrams(&line) {
                let files_vec = self.content_ngrams.entry(trigram).or_insert(Vec::new());
                files_vec.push((file_id, offset));
            }
        }
    }
}

pub fn hash_filename(filename: &str) -> u64 {
    let mut s = DefaultHasher::new();
    filename.hash(&mut s);
    s.finish()
}
