use std::fs::{read_dir, Metadata, ReadDir};
use std::path::{Path, PathBuf};
use anyhow::{anyhow, bail, Context};
use std::collections::VecDeque;

pub struct DirWalkParameters {
    pub root_dir: PathBuf,
    pub recursive: bool,
    pub filter: Option<fn(&Path) -> bool>,
}

pub struct DirWalk {
    remain: Vec<EntryList>,
    parameters: DirWalkParameters,
}

impl DirWalk {
    pub fn new(parameters: DirWalkParameters) -> Option<DirWalk> {
        let Ok(entries) = from_dir_entry(&parameters.root_dir) else {
            return None;
        };

        return Some(DirWalk {
            remain: vec![entries],
            parameters: parameters,
        });
    }

    pub fn new_recursive(root_dir: &Path) -> Option<DirWalk> {
        return DirWalk::new(DirWalkParameters {
            root_dir: root_dir.to_owned(),
            recursive: true,
            filter: None,
        });
    }
}

//TODO: currently we use PathBuf which is slower but easier to handle
impl Iterator for DirWalk {
    type Item = PathBuf;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        while !self.remain.is_empty() {
            let dir_entry = self.remain.last_mut().unwrap();

            let Some(entry) = dir_entry.pop_front() else {
                self.remain.pop();
                continue;
            };

            let is_dir = entry.metadata.is_dir();

            if is_dir && self.parameters.recursive {
                let next_entry = from_dir_entry(&entry.path);
                if let Ok(next_entry) = next_entry {
                    self.remain.push(next_entry);
                }
            }

            if let Some(filter) = &self.parameters.filter {
                if !filter(&entry.path) {
                    continue;
                }
            }

            return Some(entry.path);
        }

        return None;
    }
}

struct Entry {
    path: PathBuf,
    metadata: Metadata
}

type EntryList = VecDeque<Entry>;

fn from_dir_entry(path: &Path) -> anyhow::Result<EntryList> {
    let Ok(entries) = read_dir(&path) else {
        return Err(anyhow!("cannot read dir {}", path.to_string_lossy()));
    };

    let mut ret = EntryList::new();

    for entry in entries {
        let entry = entry.expect("cannot access the result of a read_dir");
        let metadata = entry.metadata().expect("read metadata failed");
        ret.push_back(Entry{path: entry.path(), metadata});
    }

    ret.make_contiguous().sort_by(|a,b| a.path.partial_cmp(&b.path).unwrap() );

    return Ok(ret);
}