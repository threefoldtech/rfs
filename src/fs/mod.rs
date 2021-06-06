use crate::meta;
use crate::meta::types::*;

use anyhow::Result;
use fuse::Request;
use libc::{c_int, EBADF, EIO, ENOENT};
use lru::LruCache;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::ops::{Deref, DerefMut};
use time::Timespec;
mod dn;

struct Counter<T> {
    t: T,
    count: u32,
}

impl<T> Counter<T> {
    fn from(t: T) -> Counter<T> {
        Counter { t, count: 1 }
    }

    fn incr(c: &mut Counter<T>) {
        c.count += 1;
    }

    fn decr(c: &mut Counter<T>) -> u32 {
        c.count -= 1;
        c.count
    }
}

impl<T> Deref for Counter<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.t
    }
}

impl<T> DerefMut for Counter<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.t
    }
}

pub struct Filesystem<'a> {
    meta: &'a meta::Manager,
    ttl: Timespec,
    dirs: LruCache<meta::Inode, Dir>,
    //entries: LruCache<meta::Inode, Vec<Entry>>,
    dn: dn::Manager,
    fds: HashMap<u64, Counter<dn::Chain>>,
}

fn get_dir<'c>(
    meta: &meta::Manager,
    cache: &'c mut lru::LruCache<meta::Inode, Dir>,
    inode: meta::Inode,
) -> Option<&'c Dir> {
    if cache.get(&inode).is_none() {
        if let Ok(dir) = meta.get_dir(inode) {
            cache.put(inode, dir);
        } else {
            return None;
        }
    }

    return cache.get(&inode);
}

impl<'a> Filesystem<'a> {
    /// creates a new instance of the filesystem
    pub fn new(meta: &'a meta::Manager, hub: &str, cache: &str) -> Result<Filesystem<'a>> {
        let client = redis::Client::open(hub)?;
        let downloader = dn::Manager::new(num_cpus::get(), cache, client)?;

        Ok(Filesystem {
            meta: meta,
            ttl: Timespec::new(30, 0),
            dirs: lru::LruCache::new(100),
            //entries: lru::LruCache::new(100),
            dn: downloader,
            fds: HashMap::new(),
        })
    }

    fn get_dir(&mut self, inode: meta::Inode) -> std::result::Result<Dir, c_int> {
        let inode = inode.dir();
        let dir = match get_dir(self.meta, &mut self.dirs, inode) {
            Some(dir) => dir,
            None => {
                return Err(ENOENT);
            }
        };

        Ok(dir.clone())
    }

    fn get_entry(&mut self, inode: meta::Inode) -> Result<meta::Either, c_int> {
        let dir = self.get_dir(inode)?;
        let index = inode.index() as usize;
        match index {
            0 => Ok(Either::dir(dir)),
            _ if index <= dir.entries.len() => Ok(Either::entry(dir.entries[index - 1].clone())),
            _ => Err(ENOENT),
        }
    }

    fn get_entry_by_name(&mut self, parent: meta::Inode, name: &str) -> Result<Entry, c_int> {
        let dir = self.get_dir(parent)?;
        for entry in dir.entries.iter() {
            if entry.name != name {
                continue;
            }

            return Ok(entry.clone());
        }

        Err(ENOENT)
    }
}

impl<'a> fuse::Filesystem for Filesystem<'a> {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        info!("Initializing file system");
        Ok(())
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuse::ReplyDirectory,
    ) {
        let inode = self.meta.get_inode(ino);
        let dir = match self.get_dir(inode) {
            Ok(dir) => dir,
            Err(err) => {
                reply.error(err);
                return;
            }
        };

        let header: Vec<Entry> = vec![
            Entry {
                inode: dir.inode,
                name: ".".to_string(),
                size: 0,
                acl: String::new(),
                modification: 0,
                creation: 0,
                kind: EntryKind::Unknown,
            },
            Entry {
                inode: dir.parent,
                name: "..".to_string(),
                size: 0,
                acl: String::new(),
                modification: 0,
                creation: 0,
                kind: EntryKind::Unknown,
            },
        ];

        let to_skip = if offset == 0 { offset } else { offset + 1 } as usize;
        for (index, entry) in header
            .iter()
            .chain(dir.entries.iter())
            .enumerate()
            .skip(to_skip)
        {
            if reply.add(
                entry.inode.ino(),
                index as i64,
                entry.node_type(),
                OsStr::new(&entry.name),
            ) {
                break;
            };
        }

        reply.ok();
    }

    /// Look up a directory entry by name and get its attributes.
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: fuse::ReplyEntry) {
        let name = match name.to_str() {
            Some(name) => name,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        let inode = self.meta.get_inode(parent);
        let entry = match self.get_entry_by_name(inode, name) {
            Ok(entry) => entry,
            Err(err) => {
                reply.error(err);
                return;
            }
        };

        match &entry.kind {
            EntryKind::Dir(_dir) => match get_dir(self.meta, &mut self.dirs, entry.inode) {
                Some(dir) => reply.entry(&self.ttl, &dir.attr(), 1),
                None => reply.error(ENOENT),
            },
            _ => reply.entry(&self.ttl, &entry.attr(), 1),
        };
    }

    /// Get file attributes.
    fn getattr(&mut self, _req: &Request, ino: u64, reply: fuse::ReplyAttr) {
        let inode = self.meta.get_inode(ino);

        let entry = match self.get_entry(inode) {
            Ok(entry) => entry,
            Err(code) => {
                reply.error(code);
                return;
            }
        };

        reply.attr(&self.ttl, &entry.attr());
        return;
    }

    /// Read symbolic link.
    fn readlink(&mut self, _req: &Request, ino: u64, reply: fuse::ReplyData) {
        let inode = self.meta.get_inode(ino);
        let entry = match self.get_entry(inode) {
            Ok(result) => result,
            Err(err) => {
                reply.error(err);
                return;
            }
        };

        match entry.kind() {
            EntryKind::Link(l) => {
                let mut target: String = l.target.clone();
                target.push('\0');
                reply.data(l.target.as_ref());
            }
            _ => reply.error(ENOENT),
        }
    }

    fn open(&mut self, _req: &Request, ino: u64, flags: u32, reply: fuse::ReplyOpen) {
        let inode = self.meta.get_inode(ino);
        if let Some(fd) = self.fds.get_mut(&inode.ino()) {
            Counter::incr(fd);
            reply.opened(inode.ino(), flags);
            return;
        }

        let entry = match self.get_entry(inode) {
            Ok(result) => result,
            Err(err) => {
                reply.error(err);
                return;
            }
        };

        match entry.kind() {
            EntryKind::File(f) => {
                let fd = match self.dn.open(&f) {
                    Ok(fd) => fd,
                    Err(_) => {
                        reply.error(EBADF);
                        return;
                    }
                };

                self.fds.insert(inode.ino(), Counter::from(fd));
                reply.opened(inode.ino(), flags);
            }
            _ => reply.error(ENOENT),
        }
    }

    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
        reply: fuse::ReplyEmpty,
    ) {
        let count = match self.fds.get_mut(&fh) {
            Some(fd) => Counter::decr(fd),
            None => {
                reply.ok();
                return;
            }
        };

        if count == 0 {
            debug!("releasing file handler {}", fh);
            self.fds.remove(&fh);
        }

        reply.ok();
    }

    fn read(
        &mut self,
        _req: &Request,
        _ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        reply: fuse::ReplyData,
    ) {
        let fd = match self.fds.get_mut(&fh) {
            Some(fd) => fd,
            None => {
                reply.error(EBADF);
                return;
            }
        };

        //let mut buf = [0; size];
        let mut buf: Vec<u8> = vec![0; size as usize]; //Vec::with_capacity(size as usize);

        let read = match fd.read_offset(offset as u64, &mut buf) {
            Ok(read) => read,
            Err(_) => {
                reply.error(EIO); //probably change to something else
                return;
            }
        };
        debug!("read {} bytes", read);
        reply.data(&buf[..read]);
    }
}
