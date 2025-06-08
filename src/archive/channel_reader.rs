use std::path::Path;
use std::{collections::VecDeque, fs::File, iter::Peekable, path::PathBuf};
use anyhow::{anyhow, bail, Context};
use crate::meta_format;
use crate::checksum::{self, HashResult};
use super::defs;
use super::session::{self, GetSession};
use super::session::{BackupSession, ToSession};



pub struct ChannelReaderOptions {
    pub channel: String,
    pub entry: Option<String>,
}

pub struct ChannelReader {
    session: BackupSession,

    #[allow(dead_code)]
    channel: String,

    reader: Peekable<meta_format::Reader<File>>,
    seen_entries: VecDeque<meta_format::ReaderEntry>,
    unseen_entries: Option<meta_format::ReaderEntry>,
}

//make an variant with all members optional
#[derive(Debug)]
pub struct ChannelReaderItem {
    pub relative_path: PathBuf,
    pub content_path: PathBuf,
}

impl ChannelReader {
    pub fn new(
        backup_session: BackupSession,
        opt: ChannelReaderOptions,
    ) -> anyhow::Result<ChannelReader> {
        let entry = match opt.entry {
            Some(entry) => entry.clone(),
            None => defs::channel_rev_last(&backup_session.archive_dir, &opt.channel)?
                .to_string_lossy()
                .to_string(),
        };

        let file = File::open(defs::channel_file(
            &backup_session.archive_dir,
            &opt.channel,
            &entry,
        ))?;

        return Ok(ChannelReader {
            session: backup_session,
            channel: opt.channel,
            reader: meta_format::Reader::new(file).peekable(),
            seen_entries: VecDeque::new(),
            unseen_entries: None,
        });
    }

    fn finish(&mut self) -> anyhow::Result<ChannelReaderItem> {
        let seen = std::mem::replace(&mut self.seen_entries, VecDeque::new());

        if let Some(entry) = self.unseen_entries.take() {
            self.seen_entries.push_back(entry);
        }

        let mut relative_path: Option<PathBuf> = None;
        let mut checksum: Option<HashResult> = None;

        for entry in seen {
            if entry.key == defs::keys::FILE {
                relative_path = Some(Path::new(&entry.value).into());
            } else if entry.key == defs::keys::HASH {
                checksum = Some(HashResult::from_hex_string(&entry.value)?);
            }
        }

        let item = ChannelReaderItem {
            relative_path: relative_path.ok_or(anyhow!("relative path missing"))?,
            content_path: PathBuf::from({
                let checksum = checksum.ok_or(anyhow!("checksum is missing"))?;
                defs::content_file(self.session.get_archive_dir(), &checksum.data())
            }),
        };

        return Ok(item);
    }
}

impl Iterator for ChannelReader {
    type Item = anyhow::Result<ChannelReaderItem>;

    fn next(&mut self) -> Option<anyhow::Result<ChannelReaderItem>> {
        for entry in &mut self.reader {
            let is_file = entry.key == defs::keys::FILE;
            let entry_count = self.seen_entries.len();

            if is_file && entry_count > 0 {
                self.unseen_entries = Some(entry.clone());
                return Some(self.finish());
            } else if is_file && entry_count == 0 || !is_file {
                self.seen_entries.push_back(entry.clone());
            }
        }

        if self.seen_entries.is_empty() {
            return None;
        } else {
            return Some(self.finish());
        }
    }
}

impl ToSession for ChannelReader {
    fn to_session(self) -> BackupSession { 
        return self.session;
    } 
}

impl<'a> GetSession<'a> for ChannelReader {
    fn get_session(&'a self) -> &'a BackupSession {
        return &self.session;
    }
}