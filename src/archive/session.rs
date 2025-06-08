use std::{fs, path::{Path, PathBuf}};
use anyhow::{anyhow, bail, Context};
use crate::misc_helper::{self, is_dir, is_file_or_dir};
use super::{defs::*, ContentSettings};

pub struct BackupSession {
    pub archive_dir: PathBuf,
    pub settings: ContentSettings,
   
    #[allow(dead_code)]
    pub lock: ArchiveLock,
}

impl BackupSession {
    pub fn init_session(archive_dir: &Path, settings: ContentSettings) -> anyhow::Result<()> {
        let err_msg = || {
            return format!("init archive failed");
        };
    
        if is_file_or_dir(&archive_dir) {
            return Err(anyhow!(err_msg()));
        }
    
        fs::create_dir(&archive_dir)
            .with_context(|| err_msg())?;
        fs::create_dir(&archive_dir.to_path_buf().join(CHANNEL_DIR))
            .with_context(|| err_msg())?;
        fs::create_dir(content_dir(&archive_dir.to_path_buf()))
            .with_context(|| err_msg())?;
    
        fs::write(settings_file(&archive_dir), serde_json::to_string_pretty(&settings)?)
            .with_context(||{ anyhow!("cannot write settings file {}", 
                settings_file(&archive_dir).to_string_lossy()) })?;

        return Ok(());
    }

    pub fn new(archive_dir: &Path) -> anyhow::Result<BackupSession> {
        misc_helper::is_dir_expected(&archive_dir, || "archive dir does not exist".into())?;
        misc_helper::is_dir_expected(&archive_dir.to_path_buf().join(CHANNEL_DIR), || "channel dir does not exist".into())?;
        misc_helper::is_dir_expected(&content_dir(&archive_dir.to_path_buf()), || "content dir does not exist".into())?;

        let settings= {
            let content = fs::read(settings_file(&archive_dir))
                .with_context(||{anyhow!("cannot read settings file")})?;
            let content = String::from_utf8(content)
                .with_context(||{anyhow!("settings file is not valid Utf-8")})?;
            serde_json::from_str(&content)?
        };
        
        return Ok(BackupSession {
            archive_dir: archive_dir.to_owned(),
            settings: settings,
            lock: ArchiveLock::new(archive_dir.to_owned())?,
        });
    }

    pub fn get_archive_dir(&self) -> &Path {
        return &self.archive_dir;
    }

    pub fn get_settings(&self) -> &ContentSettings {
        return &self.settings;
    }

    pub fn channel_names(&self) -> anyhow::Result<Vec<String>> {
        let mut ret: Vec<String> = Vec::new();

        for path in channel_paths(&self.archive_dir)? {
            ret.push(
                path.iter()
                    .last()
                    .ok_or(anyhow!("cannot unpack channel name"))?
                    .to_string_lossy()
                    .to_string(),
            );
        }

        return Ok(ret);
    }
}

struct ArchiveLock {
    archive_dir: Option<PathBuf>,
}

impl ArchiveLock {
    pub fn new(archive_dir: PathBuf) -> anyhow::Result<ArchiveLock> {
        fs::File::create_new(lock_file(&archive_dir))
            .with_context(|| "backup storage is locked")?;

        return Ok(ArchiveLock {
            archive_dir: Some(archive_dir),
        });
    }

    pub fn unlock(&mut self) {
        let Some(archive_dir) = &self.archive_dir else {
            return;
        };

        if let Ok(_) = fs::remove_file(lock_file(&archive_dir)) {
            self.archive_dir = None;
        } else {
            println!("cannot unlock {}", archive_dir.to_string_lossy());
        }
    }
}

impl Drop for ArchiveLock {
    fn drop(&mut self) {
        self.unlock();
    }
}

pub trait ToSession {
    fn to_session(self) -> BackupSession;
}

pub trait GetSession<'a> {
    fn get_session(&'a self) -> &'a BackupSession;
}