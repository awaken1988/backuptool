use anyhow::anyhow;
use itertools::Itertools;
use sha2::{digest::FixedOutputReset, Digest, Sha256};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

use crate::checksum::{self, HashResult};

mod reserved_keywords {
    pub const SEPERATOR: &str = ":";
    pub const END_MARKER: &str = "__end";
}

pub struct Reader<T: Read> {
    reader: BufReader<T>,
    depth: usize,
}

#[derive(Clone, Debug)]
pub struct ReaderEntry {
    pub key: String,
    pub value: String,
    pub depth: usize,
}

impl<T: Read> Reader<T> {
    pub fn new(reader: T) -> Reader<T> {
        return Reader {
            reader: BufReader::new(reader),
            depth: 0,
        };
    }

    fn key_value_dept(line: &str) -> Option<ReaderEntry> {
        let mut ch = line.chars();

        let depth = ch.by_ref().take_while_ref(|x| *x == '\t').count();

        let key = ch
            .by_ref()
            .take_while_ref(|x| *x != ':')
            .collect::<String>();

        if !ch.by_ref().next().is_some_and(|c| c == ':') {
            return None;
        }

        let value = {
            let mut value = ch.by_ref().collect::<String>();
            if value.chars().last().is_some_and(|c| c == '\n') {
                value.pop();
            }
            value
        };

        return Some(ReaderEntry {
            key: key,
            value: value,
            depth: depth,
        });
    }
}

impl<T: Read> Iterator for Reader<T> {
    type Item = ReaderEntry;

    fn next(&mut self) -> Option<ReaderEntry> {
        let mut line = String::new();

        let Ok(count) = self.reader.read_line(&mut line) else {
            panic!("MetaFormat: cannot read line");
        };

        if count == 0 {
            return None;
        }

        let entry = Reader::<T>::key_value_dept(&line).expect("MetaFormat: invalid line");

        if self.depth.abs_diff(entry.depth) > 1 {
            panic!("MetaFormat: every element need a parent element");
        }

        return Some(entry);
    }
}

pub struct Writer<T: Write> {
    writer: BufWriter<T>,
    depth: usize,
    any_writes: bool,
    digest: Sha256,
    bytes_written: usize,
}

impl<T: Write> Writer<T> {
    pub fn new(writer: T) -> Writer<T> {
        return Writer {
            writer: BufWriter::new(writer),
            depth: 0,
            any_writes: false,
            digest: Sha256::new(),
            bytes_written: 0,
        };
    }

    fn write_raw(&mut self, text: &str) -> anyhow::Result<()> {
        Digest::update(&mut self.digest, text.as_bytes());
        self.writer.write(text.as_bytes())?;
        self.writer.write(b"\n")?;
        self.bytes_written += text.as_bytes().len();
        return Ok(());
    }

    pub fn add_entry(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        let indent = std::iter::repeat('\t').take(self.depth).collect::<String>();
        self.write_raw(&format!("{}{}:{}", indent, key, value))?;
        return Ok(());
    }

    pub fn increase_depth(&mut self) {
        self.depth += 1;
        self.any_writes = true;
    }

    pub fn decrease_depth(&mut self) {
        if self.depth == 0 {
            panic!("MetaFormat Writer: invalid depth")
        }
        self.depth -= 1;
        self.any_writes = true;
    }
}

impl<T: Write> Drop for Writer<T> {
    fn drop(&mut self) {
        if self.bytes_written == 0 {
            return;
        }

        let hashsum = self.digest.finalize_fixed_reset();
        let hashsum = HashResult::from_data(&hashsum.to_vec());

        self.write_raw(&format!(
            "{}{}{}",
            reserved_keywords::END_MARKER,
            reserved_keywords::SEPERATOR,
            hashsum.to_string()
        ))
        .expect("meta format cannot write end marker");
    }
}

pub fn verify<T: Read>(reader: T) -> anyhow::Result<()> {
    let mut hasher = Sha256::new();
    let mut read_hashsum = None;

    for line in BufReader::new(reader).lines() {
        let line = line?;

        if let Some((_, r)) = line.split_once(&(reserved_keywords::END_MARKER.to_owned() + ":")) {
            read_hashsum = Some(r.to_owned());
            break;
        }

        hasher.update(line.as_bytes());
    }

    let Some(read_hashsum) = read_hashsum else {
        return Err(anyhow!(
            "cannot read {} marker",
            reserved_keywords::END_MARKER
        ));
    };

    let read_hashsum = HashResult::from_hex_string(&read_hashsum)?;
    let calc_hashsum = HashResult::from_data(&hasher.finalize().to_vec());

    if read_hashsum.data() == calc_hashsum.data() {
        return Ok(());
    } else {
        return Err(anyhow!("hashsum mismatch"));
    }
}
