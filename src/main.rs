#![allow(warnings)]

//TODO: archive_dir, data_dir

mod archive;
mod checksum;
mod dirwalk;
mod meta_format;
mod misc_helper;
mod test;


use archive::{BackupSession, ChannelReader, ChannelReaderOptions, ChannelWriter, ChannelWriterAdd,ContentCompression, GetSession};
use checksum::HashAlgo;
use clap::{Parser, Subcommand};
use crossbeam;
use dirwalk::{DirWalk, DirWalkParameters};
use misc_helper::CopyAction;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use anyhow::{anyhow, bail, Context};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Path of the backup archive
    #[arg(short, long)]
    archive: String,

    /// write source files to the archive
    #[command(subcommand)]
    subcommands: Option<SubCli>,
}

#[derive(Subcommand)]
enum SubCli {
    /// create a new arhive
    New,

    /// Write files from a source dir to archive
    Backup {
        /// path to dir which will be backed
        #[arg(short, long)]
        source: String,

        /// channel name for the archive dir
        #[arg(short, long)]
        channel: String,
    },

    /// Restore a specific revision of a channel to a destination folder
    Restore {
        /// path to dir where the content restored
        #[arg(short, long)]
        destination: String,

        /// channel name for the source
        #[arg(short, long)]
        channel: String,

        /// entry name
        #[arg(short, long)]
        entry: Option<String>,
    },

    /// Verify the integrity of the archive
    Verify,

    /// List all channels
    ListChannel {
        /// todo
        #[arg(short, long)]
        todo: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {


    //test new dirwalk
    // {
    //     let walker = DirWalk::new(DirWalkParameters{
    //         root_dir: "/home/martin/src/acme/tools".into(),
    //         recursive: true,
    //         filter: None}).unwrap();
    //     for i in walker {
    //         println!("{:?}", i)
    //     }
    //     return Ok(());
    // }

    //#delete
    // {
    //     let dirwalk = DirWalk::new_recursive(
    //         &PathBuf::from(r#"C:\dev\tools_and_snippets\"#))
    //         .unwrap();
    //     for i in dirwalk {
    //         print!("{:?}\n", i)
    //     }
    //     return Ok(());
    // }


    let cli = Cli::parse();

    match &cli.subcommands.unwrap() {
        SubCli::New => {
            return BackupSession::init_session(
                &PathBuf::from(cli.archive),
                archive::ContentSettings {
                    compression: ContentCompression::Bzip2 { level: 1 },
                    hash_algo: HashAlgo::Sha256,
                });
        },
        SubCli::Backup { source, channel } => {
            let session = BackupSession::new(&PathBuf::from(cli.archive))?;
            let channel_writer = ChannelWriter::new(session, channel)?;
            return backup_command(&PathBuf::from(source), channel_writer);
        }
        SubCli::Restore {
            destination,
            channel,
            entry,
        } => {
            let session = BackupSession::new(&PathBuf::from(cli.archive))?;
            let channel_reader = ChannelReader::new(session, ChannelReaderOptions {
                channel: channel.clone(),
                entry: entry.clone(),
            })?;

            return restore(channel_reader, &PathBuf::from(&destination));
        }
        SubCli::Verify => {
            let session = BackupSession::new(&PathBuf::from(cli.archive))?;

            return Ok(());
        }
        SubCli::ListChannel { todo: _ } => {
            let session = BackupSession::new(&PathBuf::from(cli.archive))?;

            for channel in session.channel_names()? {
                println!("{}", channel);
            }

            return Ok(());
        }
    };

    return Ok(());
}

pub fn restore(channel_reader: archive::ChannelReader, restore_dir: &Path) -> anyhow::Result<()> {
    for backup_info in channel_reader {
        let Ok(backup_info) = backup_info else {
            println!("RESTORE error entry");
            continue;
        };

        let restore_file = PathBuf::new()
            .join(&restore_dir)
            .join(&backup_info.relative_path);

        let restore_subdirs = restore_file.parent().unwrap();
        misc_helper::create_dir_when_missing(restore_subdirs).unwrap();

        if misc_helper::is_file_or_dir(&restore_file) {
            println!(
                "restore {:?} restore file or dir already exists",
                &backup_info.relative_path
            );
        } else {
            println!("restore {:?}", &backup_info.relative_path);
        }

        misc_helper::copy_convert(
            &backup_info.content_path,
            &restore_file,
            CopyAction::UnCompress,
        )?;
    }

    return Ok(());
}

fn backup_file(
    channel_writer: Arc<Mutex<ChannelWriter>>,
    file_path: &Path,
    base_dir: &Path,
) -> anyhow::Result<()> {
    let Ok(metadata) = file_path.metadata() else {
        println!("cannot get metadata: {}", file_path.to_string_lossy());
        return Ok(());
    };

    let settings = channel_writer
        .lock()
        .expect("cannot get settings from runtime session")
        .get_session()
        .get_settings()
        .clone();

    if metadata.is_file() {
        let Ok(checksum) = checksum::new_hasher(settings.hash_algo).file(&file_path, ) else {
            println!("cannot checksum file: {}", file_path.to_string_lossy());
            return Ok(());
        };
        let checksum_str = checksum.to_string_short();

        let action = channel_writer.lock()
            .expect("writer worker error; cannot lock writer")
            .deref_mut()
            .add_file(
                misc_helper::relative_path(base_dir.as_ref(), file_path.as_ref()).as_path(),
                &checksum,
            )?;

        match action {
            ChannelWriterAdd::HashFile(hash_path) => {
                println!("new file    {}    {}", checksum_str, file_path.to_string_lossy());
                misc_helper::copy_convert(&file_path, &hash_path, CopyAction::Compress)?;
            }
            ChannelWriterAdd::AlreadyExist => {
                println!("skip file   {}    {}", checksum_str, file_path.to_string_lossy());
            }
        };

            
    } else if metadata.is_dir() {
        println!("dir         {}", file_path.to_string_lossy());
        channel_writer.lock()
            .expect("writer worker error; cannot lock writer")
            .deref_mut()
            .add_dir(file_path)?
    }
    else {
        println!("invalid     {}", file_path.to_string_lossy());
    };

    return Ok(());
}

pub fn backup_command(
    src_dir: &Path,
    channel_writer: ChannelWriter,
) -> anyhow::Result<()> {
    let thread_count = 4usize;
    let channel_writer = Arc::new(Mutex::new(channel_writer));

    let (send_channel, recv_channel) = crossbeam::channel::bounded::<Option<PathBuf>>(1);

    let mut join_handles = Vec::new();
    for _ in 0..thread_count {
        let channel_writer = channel_writer.clone();
        let recv_channel = recv_channel.clone();
        let src_dir = src_dir.to_owned();
        let handle = thread::spawn(move || {
            while let Some(src_file) = recv_channel.recv().unwrap() {
                backup_file(channel_writer.clone(), &src_file, &src_dir).unwrap();
            }
        });

        join_handles.push(handle);
    }

    for entry in DirWalk::new_recursive(src_dir).unwrap() {
        send_channel.send(Some(entry))?;
    }
    for _ in join_handles.iter() {
        send_channel.send(None)?;
    }
    for handle in join_handles {
        handle.join().expect("join worker failed");
    }

    return Ok(());
}
