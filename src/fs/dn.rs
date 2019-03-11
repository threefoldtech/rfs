use crate::meta::types;
use crossbeam::scope;
use fs2::FileExt;
//use redis;
use snappy::uncompress;
use std::fmt;
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path;
use std::thread;
use std::time;

trait Hex {
    fn hex(self: &Self) -> String;
}

impl Hex for Vec<u8> {
    fn hex(&self) -> String {
        self.iter()
            .map(|x| -> String { format!("{:02x}", x) })
            .collect()
    }
}

const cache_dir: &str = "/tmp/cache";

#[derive(Debug)]
pub struct DownloadError {
    message: String,
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DownloadError {}

type Result<T> = std::result::Result<T, DownloadError>;

pub struct Manager<'a> {
    size: usize,
    client: &'a redis::Client,
}

impl<'a> Manager<'a> {
    pub fn new(size: usize, client: &'a redis::Client) -> Manager<'a> {
        Manager { size, client }
    }

    fn get_chunk(&self, name: String) -> std::io::Result<fs::File> {
        fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path::Path::new(cache_dir).join(name))
    }

    /// check_and_get runs inside a scoped thread, it communicate
    /// errors with panics!
    fn check_and_get(&self, block: &types::FileBlock) -> fs::File {
        let name = block.Hash.hex();
        let mut file = self.get_chunk(name).unwrap();
        file.lock_exclusive().unwrap();
        //TODO: check hash ?
        if file.metadata().unwrap().len() > 0 {
            file.unlock().unwrap();
            return file;
        }

        debug!("getting file chunk {}", block.Hash.hex());
        let con = self.client.get_connection().unwrap();
        let mut result: Vec<u8> = redis::cmd("get")
            .arg(block.Hash.to_vec())
            .query(&con)
            .unwrap();
        //let key: &str = block.Key.as_ref();
        //let key = std::str::from_utf8(&block.Key).unwrap();
        let key = unsafe { std::str::from_utf8_unchecked(&block.Key) };
        let mut result = uncompress(&xxtea::decrypt(&result, key)).unwrap();
        debug!(
            "writing file chunk {} (size: {})",
            block.Hash.hex(),
            result.len()
        );

        file.write_all(&mut result).unwrap();
        file.sync_all().unwrap();
        file.unlock().unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();

        file
    }

    pub fn download(&self, object: &types::FileEntry) -> Result<Vec<fs::File>> {
        let blocks = &object.blocks;
        let mut w = blocks.len() / self.size;
        if blocks.len() % self.size > 0 {
            w += 1;
        }

        let result = scope(|sc| -> Vec<fs::File> {
            let mut handlers = vec![];
            for id in 0..self.size {
                let s = id * w;
                if s >= blocks.len() {
                    break;
                }
                let mut e = s + w;
                if id == self.size - 1 {
                    e = blocks.len();
                }

                let slice = &blocks[s..e];
                let h = sc.spawn(move |_| -> Vec<fs::File> {
                    let mut files: Vec<fs::File> = vec![];
                    for block in slice.iter() {
                        files.push(self.check_and_get(block));
                    }
                    files
                });

                handlers.push(h);
            }

            let mut files: Vec<fs::File> = vec![];

            for h in handlers {
                match h.join() {
                    Ok(mut fds) => files.append(&mut fds),
                    Err(_) => {
                        //do nothing here.
                    }
                }
            }

            files
        });

        match result {
            Ok(files) => Ok(files),
            Err(err) => Err(DownloadError {
                message: format!("{:?}", err),
            }),
        }
    }
}
