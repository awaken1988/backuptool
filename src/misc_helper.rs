//Misc helper
//  later move this to another place

use anyhow::anyhow;
use bzip2::bufread::BzDecoder;
use bzip2::read::BzEncoder;
use bzip2::Compression;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

pub fn relative_path(base_path: &Path, sub_path: &Path) -> PathBuf {
    assert_eq!(sub_path.starts_with(base_path), true);

    let mut sub_iter = sub_path.components().into_iter();

    for _ in base_path.components() {
        sub_iter.next();
    }

    let mut relative = PathBuf::new();

    while let Some(sub_component) = sub_iter.next() {
        relative.push(sub_component);
    }

    return relative;
}

const fn mebibyte(value: usize) -> usize {
    return value * 1024 * 1024;
}

pub const BUFFER_SIZE: usize = mebibyte(1);

pub fn is_file(path: &Path) -> bool {
    return path.metadata().map_or(false, |x| x.is_file());
}

pub fn is_dir(path: &Path) -> bool {
    return path.metadata().map_or(false, |x| x.is_dir());
}

pub fn is_file_or_dir(path: &Path) -> bool {
    return path.metadata().map_or(false, |x| x.is_file() | x.is_dir());
}

pub fn is_dir_expected(path: &Path, err_msg: fn() -> String) -> anyhow::Result<()> { 
    return match is_dir(path) {
        true => Ok(()),
        false => Err(anyhow!(err_msg())),
    };
}

pub fn create_dir_when_missing(path: &Path) -> anyhow::Result<()> {
    if is_dir(path) {
        return Ok(());
    }

    fs::create_dir_all(path)?;

    return Ok(());
}

pub enum CopyAction {
    Compress,
    UnCompress,
}

pub fn copy_convert(src: &Path, dst: &Path, options: CopyAction) -> anyhow::Result<()> {
    let Ok(src_file) = File::open(src) else {
        return Err(anyhow!("cannot open source file {}", src.to_string_lossy()));
    };
    let Ok(dst_file) = File::create(&dst) else {
        return Err(anyhow!(
            "cannot open destination file {}",
            src.to_string_lossy()
        ));
    };

    let mut src_file = match options {
        CopyAction::Compress => {
            Box::new(BzEncoder::new(src_file, Compression::best())) as Box<dyn Read>
        }
        CopyAction::UnCompress => {
            Box::new(BzDecoder::new(BufReader::new(src_file))) as Box<dyn Read>
        }
    };

    let mut dst_file = BufWriter::new(dst_file);

    // create out file
    let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];

    loop {
        let Ok(read_size) = src_file.read(&mut buffer[..]) else {
            return Err(anyhow!(
                "cannot read from source file {}",
                src.to_string_lossy()
            ));
        };

        if read_size == 0 {
            break;
        }

        dst_file.write(&buffer[0..read_size])?;
    }

    return Ok(());
}

pub fn print_error_chain(err: &anyhow::Error) {
    let mut indent = 0usize;
    for cause in err.chain() {
        for _ in 0..indent {
            eprint!("\t");
        }
        eprintln!("{}", cause);
        indent += 1;
    }
}
