use std::io::{BufWriter, BufReader};
use std::io::{Write, Read};
use bzip2::write::BzEncoder;
use bzip2::read::BzDecoder;
use bzip2::{self, Compression};
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use serde_json::Result;

use crate::checksum::HashAlgo;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum ContentCompression {
    None,
    Bzip2{level: u32},
}
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct ContentSettings {
    pub compression: ContentCompression,
    pub hash_algo: HashAlgo,
}

fn create_compression<T: Write + 'static> (outer_writer: T, format: &ContentCompression) -> Box<dyn Write> {
    return match format {
        ContentCompression::None => 
            Box::new(BufWriter::new(outer_writer)) as Box<dyn Write>,
        ContentCompression::Bzip2 { level } => 
            Box::new(BzEncoder::new(BufWriter::new(outer_writer), Compression::new(*level))) as Box<dyn Write>,
    };
}

fn create_decompression<T: Read + 'static> (outer_reader: T, format: &ContentCompression) -> Box<dyn Read> {
    return match format {
        ContentCompression::None => 
            Box::new(BufReader::new(outer_reader)) as Box<dyn Read>,
        ContentCompression::Bzip2 { level } => 
            Box::new(BzDecoder::new(BufReader::new(outer_reader))) as Box<dyn Read>,
    };
} 

pub struct ContentWriter
{
    writer: Box<dyn Write>,
    count: u64,
}

impl ContentWriter {
    fn new(outer_writer: Box<dyn Write>, settings: &ContentSettings) -> ContentWriter {
        return ContentWriter {
            writer: create_compression(outer_writer, &settings.compression),
            count: 0,
        };
    }

    fn bytes_written(&self) -> u64 {
        return self.count;
    }
}

impl Write for ContentWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.count += buf.len() as u64;
        return self.writer.write(buf);
    }

    fn flush(&mut self) -> std::io::Result<()> {
        return self.writer.flush();
    }
}

pub struct ContentReader {
    reader: Box<dyn Read>,
    digest: Option<Sha256>,
}

impl ContentReader {
    pub fn new(outer_reader: Box<dyn Read>, settings: &ContentSettings, with_hash: bool) -> ContentReader {
        return ContentReader {
            reader: create_decompression(outer_reader, &settings.compression),
            digest: match with_hash {
                true => Some(Sha256::new()),
                false => None,
            },
        };
    }

    //TODO: get hash value
}

impl Read for ContentReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let result = self.reader.read(buf);

        if result.is_err() {
            return result;
        }
        
        if let Some(digest) = &mut self.digest {
            digest.update(&buf);
        }
        
        return result;
    }
}
