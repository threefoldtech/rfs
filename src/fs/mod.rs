use crate::meta;
use crate::meta::types::*;

use fuse::Request;
use libc::{c_int, EBADF, ENOENT, ENOSYS};
use lru::LruCache;
use std::ffi::OsStr;
use time::Timespec;

mod dn;

pub struct Filesystem<'a> {
    meta: &'a meta::Manager,
    ttl: Timespec,
    dirs: LruCache<meta::Inode, Dir>,
    entries: LruCache<meta::Inode, Vec<Entry>>,
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
    pub fn new(meta: &'a meta::Manager) -> Filesystem<'a> {
        Filesystem {
            meta: meta,
            ttl: Timespec::new(30, 0),
            dirs: lru::LruCache::new(100),
            entries: lru::LruCache::new(100),
        }
    }

    fn get_entries(&mut self, inode: meta::Inode) -> Result<(&Dir, &Vec<Entry>), c_int> {
        let inode = inode.dir();
        let dir = match get_dir(self.meta, &mut self.dirs, inode) {
            Some(dir) => dir,
            None => {
                return Err(ENOENT);
            }
        };

        if self.entries.get(&inode).is_none() {
            let entries = match dir.entries(self.meta) {
                Ok(entries) => entries,
                Err(err) => {
                    return Err(ENOENT);
                }
            };
            self.entries.put(inode, entries);
        }

        let entries = match self.entries.get(&inode) {
            Some(entries) => entries,
            None => {
                //reply.error(ENOENT);
                return Err(ENOENT);
            }
        };

        Ok((dir, entries))
    }

    fn get_entry(&mut self, inode: meta::Inode) -> Result<(&Dir, Option<&Entry>), c_int> {
        let (dir, entries) = self.get_entries(inode.dir())?;
        let index = inode.index() as usize;
        match index {
            0 => Ok((dir, None)),
            _ if index <= entries.len() => Ok((dir, Some(&entries[index - 1]))),
            _ => Err(ENOENT),
        }
    }

    fn get_entry_by_name(&mut self, parent: meta::Inode, name: &str) -> Result<Entry, c_int> {
        let (_, entries) = self.get_entries(parent)?;
        for entry in entries.iter() {
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
        let (dir, entries) = match self.get_entries(inode) {
            Ok(entries) => entries,
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
                inode: dir.parent(self.meta),
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
            .chain(entries.iter())
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
            EntryKind::Dir(dir) => match get_dir(self.meta, &mut self.dirs, entry.inode) {
                Some(dir) => reply.entry(&self.ttl, &dir.attr(), 1),
                None => reply.error(ENOENT),
            },
            _ => reply.entry(&self.ttl, &entry.attr(), 1),
        };
    }

    /// Get file attributes.
    fn getattr(&mut self, _req: &Request, ino: u64, reply: fuse::ReplyAttr) {
        let inode = self.meta.get_inode(ino);
        let dir = match get_dir(self.meta, &mut self.dirs, inode) {
            Some(dir) => dir,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        let index = inode.index() as usize;
        if index == 0 {
            //dir inode
            reply.attr(&self.ttl, &dir.attr());
            return;
        }

        if self.entries.get(&inode).is_none() {
            let entries = match dir.entries(self.meta) {
                Ok(entries) => entries,
                Err(err) => {
                    reply.error(ENOENT);
                    return;
                }
            };
            self.entries.put(inode, entries);
        }

        let entries = match self.entries.get(&inode) {
            Some(entries) => entries,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if index > entries.len() {
            reply.error(ENOENT);
            return;
        }

        reply.attr(&self.ttl, &entries[index - 1].attr());
    }

    /// Read symbolic link.
    fn readlink(&mut self, _req: &Request, ino: u64, reply: fuse::ReplyData) {
        let inode = self.meta.get_inode(ino);
        let (_, entry) = match self.get_entry(inode) {
            Ok(result) => result,
            Err(err) => {
                reply.error(err);
                return;
            }
        };

        let entry = match entry {
            Some(entry) => entry,
            None => {
                reply.error(ENOSYS);
                return;
            }
        };

        match &entry.kind {
            EntryKind::Link(l) => {
                let mut target: String = l.target.clone();
                target.push('\0');
                reply.data(l.target.as_ref());
            }
            _ => reply.error(ENOENT),
        }
    }

    fn open(&mut self, _req: &Request, ino: u64, _flags: u32, reply: fuse::ReplyOpen) {
        let inode = self.meta.get_inode(ino);
        let (_, entry) = match self.get_entry(inode) {
            Ok(result) => result,
            Err(err) => {
                reply.error(err);
                return;
            }
        };

        let entry = match entry {
            Some(entry) => entry,
            None => {
                reply.error(ENOSYS);
                return;
            }
        };

        match &entry.kind {
            EntryKind::File(f) => {
                let client = match redis::Client::open("redis://hub.grid.tf:9900") {
                    Ok(client) => client,
                    Err(err) => {
                        error!("failed to create redis client {}", err);
                        reply.error(EBADF);
                        return;
                    }
                };

                let manager = dn::Manager::new(5, &client);
                manager.download(&f);
                // //download parts
                // for block in f.blocks.iter() {
                //     println!("Block Hash({:?}), Key({:?})", block.Hash, block.Key);
                // }
                reply.error(ENOSYS);
            }
            _ => reply.error(ENOENT),
        }
    }
}
