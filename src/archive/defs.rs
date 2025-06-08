use std::path::{Path, PathBuf};
use anyhow::{anyhow, bail, Context};
use clap::builder::PathBufValueParser;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use crate::dirwalk::{DirWalk, DirWalkParameters};
use crate::misc_helper;
use chrono::{Datelike, Timelike, Utc};

pub const CONTENT_DIR: &str = "content";
pub const CHANNEL_DIR: &str = "channels";
pub const LOCK_FILE: &str = "lock";
pub const SETTINGS_FILE: &str = "settings.json";
pub mod keys {
    pub const FILE: &str = "file";
    pub const DIR: &str = "dir";
    pub const HASH: &str = "hash";
}

pub fn settings_file(archive_dir:&Path) ->PathBuf {
    return archive_dir.to_path_buf().join(SETTINGS_FILE);
}

pub fn content_dir(archive_dir: &Path) -> PathBuf {
    return archive_dir.to_path_buf().join(CONTENT_DIR);
}

pub fn content_file(archive_dir: &Path, hash: &[u8]) -> PathBuf {
    let mut hash_str = String::new();
    for byte in hash {
        hash_str.push_str(&format!("{:02x}", *byte));
    }

    return content_dir(archive_dir).join(hash_str);
}

pub fn channel_dir(archive_dir: &Path, channel: &str) -> PathBuf {
    return archive_dir.to_path_buf().join(CHANNEL_DIR).join(channel);
}

pub fn channel_file(archive_dir: &Path, channel: &str, channel_rev: &str) -> PathBuf {
    return channel_dir(archive_dir, channel).join(format!("{}", channel_rev));
}

pub fn lock_file(archive_dir: &Path) -> PathBuf {
    return archive_dir.to_path_buf().join(LOCK_FILE);
}

pub fn next_channel_file(archive_dir: &Path, channel: &str) -> anyhow::Result<PathBuf> {
    let t = Utc::now();
    let rn = StdRng::from_os_rng().next_u64();

    let file_name = format!("{}{}{}", t.year(), t.month(), t.day())
        + &format!("_{:0>2}{:0>2}", t.hour(), t.minute())
        + &format!("_{:0>2}", t.second())
        + &format!("_{:016x}", rn);

    return Ok(channel_dir(archive_dir, channel).join(file_name));
}

fn content_paths(archive_dir: &Path) -> anyhow::Result<DirWalk> {
    return Ok(DirWalk::new(DirWalkParameters {
        root_dir: content_dir(archive_dir),
        recursive: false,
        filter: Some(|path| {
            return misc_helper::is_file(path);
        }),
    })
    .expect("archive missing"));
}

pub fn channel_rev_paths(archive_dir: &Path, channel: &str) -> anyhow::Result<DirWalk> {
    return Ok(DirWalk::new(DirWalkParameters {
        root_dir: channel_dir(archive_dir, channel),
        recursive: false,
        filter: Some(|path| {
            return misc_helper::is_file(path);
        }),
    })
    .expect("archive missing"));
}

pub fn channel_paths(archive_dir: &Path) -> anyhow::Result<DirWalk> {
    return Ok(DirWalk::new(DirWalkParameters {
        root_dir: archive_dir.to_path_buf().join(CHANNEL_DIR),
        recursive: false,
        filter: Some(|path| {
            return misc_helper::is_dir(path);
        }),
    })
    .expect("archive missing"));
}

pub fn channel_rev_last(archive_dir: &Path, channel: &str) -> anyhow::Result<PathBuf> {
    let mut latest_rev = None;

    for rev in channel_rev_paths(archive_dir, channel)? {
        if let Some(latest_rev) = &mut latest_rev {
            if *rev > *latest_rev {
                *latest_rev = rev;
            }
        } else {
            latest_rev = Some(rev);
        }
    }

    return latest_rev.ok_or(anyhow!("cannot get latest revision in channel {}", channel));
}


