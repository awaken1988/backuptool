use std::{fs::File, hash::Hash, io::{BufReader, Read}, path::Path};
use bzip2::read;
use hex;
use sha2::{self, Digest, Sha256};
use digest::{self, generic_array::ArrayLength, FixedOutputReset, OutputSizeUser};
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

pub const OUTPUT_SIZE_SHORT: usize = 4;

fn doit<D, T: Digest<OutputSize = D> + digest::Update>() {

}

type MyType = <Sha256 as OutputSizeUser>::OutputSize;

fn outer<>() {
    doit::<MyType, Sha256>();
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum HashAlgo {
    Sha256,
}

pub struct HashResult{
    digest: Vec<u8>,
    //TODO: maybe algo so we can check again
}
impl HashResult {
    pub fn from_data(data: &[u8]) -> HashResult {
        return HashResult {
            digest: data.into(),
        };
    }

    pub fn from_hex_string(digest: &str) -> anyhow::Result<HashResult> {
        let decoded = hex::decode(digest)?;
        return Ok(HashResult {
            digest: decoded
        });
    }

    pub fn to_string(&self) -> String {
        return hex::encode(&self.digest);
    }
    
    pub fn to_string_short(&self) -> String {
        let digit_len = self.digest.len().min(OUTPUT_SIZE_SHORT);
        return format!("{}...", hex::encode(&self.digest[0..digit_len]));
    }

    pub fn data(&self) -> &[u8] {
        return &self.digest;
    }
}

pub trait Hasher {
    fn update(&mut self, data: &[u8]);
    fn finalize(&mut self, ) -> HashResult;
}


struct DigestImpl<T> where 
    T: Digest
{
    hasher: T,
}

impl<T> DigestImpl<T>  where 
    T: Digest 
{
    fn new() -> DigestImpl<T> {
        return DigestImpl{
            hasher: T::new()
        };
    }
}

impl<T> Hasher for DigestImpl<T>  where 
    T: Digest + FixedOutputReset
{
    fn update(&mut self, data: &[u8]) {
        Digest::update(&mut self.hasher, data);
    }

    fn finalize(&mut self, ) -> HashResult {
        return HashResult{
            digest: self.hasher.finalize_reset().to_vec()
        };
    }
}

pub fn new_hasher(algo: HashAlgo) -> Box<dyn Hasher> {
    match algo {
        HashAlgo::Sha256 => { return Box::new(DigestImpl::<Sha256>::new()) as Box<dyn Hasher>; }
    }
}

impl dyn Hasher {
    pub fn stream<T: Read>(&mut self, reader: T) -> anyhow::Result<HashResult>  {
        let mut reader = BufReader::new(reader);
        let mut buffer = vec![0u8; 1024];
        loop {
            let read_size = reader.read(&mut buffer)?;
            if read_size == 0 {
                break;
            }
            self.update(&buffer[0..read_size]);
        }

        return Ok(self.finalize());
    }

    pub fn file(&mut self, path: &Path) -> anyhow::Result<HashResult> {
        return self.stream(File::open(path)?)
    }   
}

fn new_hasher_salt(algo: HashAlgo, salt: &[u8]) -> Box<dyn Hasher> {
    let mut hasher = new_hasher(algo);
    hasher.update(salt);
    return hasher;
}



