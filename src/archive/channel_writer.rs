use std::fs::File;
use std::path::Path;
use std::{io::Write, path::PathBuf};
use anyhow::{anyhow, bail, Context};
use crate::checksum::HashResult;
use crate::{checksum, meta_format, misc_helper};
use super::defs;
use super::session::{BackupSession, GetSession, ToSession};


pub struct ChannelWriter {
    session: BackupSession,
    writer: meta_format::Writer<Box<dyn Write + Send>>,
}

pub enum ChannelWriterAdd {
    HashFile(PathBuf),
    AlreadyExist,
}

impl<'a> ChannelWriter {
    pub fn new(backup_session: BackupSession, channel: &str) -> anyhow::Result<ChannelWriter> {
        let archive_dir = &backup_session.archive_dir;

        misc_helper::create_dir_when_missing(&defs::channel_dir(archive_dir, channel))?;
        misc_helper::create_dir_when_missing(&defs::content_dir(archive_dir))?;
        let file = Box::new(File::create(defs::next_channel_file(archive_dir, channel)?)?);

        return Ok(ChannelWriter {
            session: backup_session,
            writer: meta_format::Writer::new(file),
        });
    }

    pub fn add_file(
        &mut self,
        path: &Path,
        checksum: &HashResult,
    ) -> anyhow::Result<ChannelWriterAdd> {
        //meta data
        self.writer.add_entry(
            defs::keys::FILE,
            path.to_str()
                .ok_or(anyhow::Error::msg("Could not add entry"))?,
        )?;

        self.writer.increase_depth();
        self.writer.add_entry(defs::keys::HASH, &checksum.to_string())?;
        self.writer.decrease_depth();

        //file
        let target_path = defs::content_file(self.session.get_archive_dir(), &checksum.data());

        if Path::is_file(&target_path) {
            return Ok(ChannelWriterAdd::AlreadyExist);
        }

        return Ok(ChannelWriterAdd::HashFile(target_path));
    }

    pub fn add_dir(&mut self, path: &Path) -> anyhow::Result<()> {
        self.writer.add_entry(
            defs::keys::DIR,
            path.to_str()
                .ok_or(anyhow::Error::msg("Could not add entry"))?,
        )?;

        return Ok(())
    }

}

impl ToSession for ChannelWriter {
    fn to_session(self) -> BackupSession { 
        return self.session;
    } 
}

impl<'a> GetSession<'a> for ChannelWriter {
    fn get_session(&'a self) -> &'a BackupSession {
        return &self.session;
    }
}