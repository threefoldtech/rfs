use crate::meta;
use crate::meta::types::*;

use fuse::Request;
use libc::{c_int, ENOENT, ENOSYS};
use std::ffi::OsStr;
use time::Timespec;

use lru::LruCache;

pub struct Filesystem<'a> {
    meta: &'a meta::Manager,
    ttl: Timespec,
    cache: LruCache<meta::Inode, Dir>,
}

impl<'a> Filesystem<'a> {
    /// creates a new instance of the filesystem
    pub fn new(meta: &'a meta::Manager) -> Filesystem<'a> {
        Filesystem {
            meta: meta,
            ttl: Timespec::new(30, 0),
            cache: lru::LruCache::new(1000),
        }
    }
}

impl<'a> Filesystem<'a> {
    fn lookup_entry(&mut self, entry: Entry, reply: fuse::ReplyEntry) {
        match entry.kind {
            EntryKind::Dir(dir) => match Self::get_dir(self.meta, &mut self.cache, entry.inode) {
                Some(dir) => reply.entry(&self.ttl, &dir.attr(), 1),
                None => reply.error(ENOENT),
            },
            _ => reply.entry(&self.ttl, &entry.attr(), 1),
        };
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
}

impl<'a> fuse::Filesystem for Filesystem<'a> {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        info!("Initializing file system");
        Ok(())
    }

    fn readdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: fuse::ReplyDirectory,
    ) {
        let inode = self.meta.get_inode(_ino);

        let dir = match Self::get_dir(self.meta, &mut self.cache, inode) {
            Some(dir) => dir,
            None => {
                reply.error(ENOENT);
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

        let entries = match dir.entries(self.meta) {
            Ok(entries) => entries,
            Err(err) => {
                reply.error(ENOENT);
                return;
            }
        };

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
    fn lookup(&mut self, _req: &Request, _parent: u64, _name: &OsStr, reply: fuse::ReplyEntry) {
        let name = match _name.to_str() {
            Some(name) => name,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        let inode = self.meta.get_inode(_parent);
        let dir = match Self::get_dir(self.meta, &mut self.cache, inode) {
            Some(dir) => dir,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        let entries = match dir.entries(self.meta) {
            Ok(entries) => entries,
            Err(err) => {
                reply.error(ENOENT);
                return;
            }
        };

        // scan entries for the name
        for entry in entries {
            if entry.name != name {
                continue;
            }

            self.lookup_entry(entry, reply);
            return;
        }

        reply.error(ENOENT);
    }

    /// Get file attributes.
    fn getattr(&mut self, _req: &Request, _ino: u64, reply: fuse::ReplyAttr) {
        let inode = self.meta.get_inode(_ino);

        let node = match self.meta.get_node(inode) {
            Ok(node) => node,
            Err(err) => {
                reply.error(ENOENT);
                return;
            }
        };

        reply.attr(&self.ttl, &node.attr());
    }

    /// Read symbolic link.
    fn readlink(&mut self, _req: &Request, _ino: u64, reply: fuse::ReplyData) {
        let inode = self.meta.get_inode(_ino);
        let node = match self.meta.get_node(inode) {
            Ok(node) => node,
            Err(err) => {
                reply.error(ENOENT);
                return;
            }
        };
        debug!("Read link {:?}", node.attr());
        match node.kind() {
            EntryKind::Link(l) => {
                let mut target: String = l.target.clone();
                target.push('\0');
                reply.data(l.target.as_ref());
            }
            _ => reply.error(ENOENT),
        }
    }
}
