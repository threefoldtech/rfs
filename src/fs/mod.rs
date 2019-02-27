use crate::meta;
use fuse::Request;
use libc::{c_int, ENOENT, ENOSYS};
use std::ffi::OsStr;
use time::Timespec;

pub struct Filesystem<'a> {
    meta: &'a meta::Manager,
    ttl: Timespec,
}

impl<'a> Filesystem<'a> {
    /// creates a new instance of the filesystem
    pub fn new(meta: &'a meta::Manager) -> Filesystem<'a> {
        Filesystem {
            meta: meta,
            ttl: Timespec::new(30, 0),
        }
    }
}

impl<'a> Filesystem<'a> {
    fn lookup_entry(&mut self, entry: meta::types::Entry, reply: fuse::ReplyEntry) {
        match entry.kind {
            meta::EntryKind::Dir(dir) => match self.meta.get_dir_by_key(&dir.key) {
                Ok(dir) => reply.entry(&self.ttl, &dir.attr(), 1),
                Err(err) => reply.error(ENOENT),
            },
            _ => {
                reply.error(ENOENT);
                return;
            }
        };
    }
}

impl<'a> fuse::Filesystem for Filesystem<'a> {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        info!("Initializing file system");
        Ok(())
    }

    // fn opendir(&mut self, _req: &fuse::Request, _ino: u64, _flags: u32, reply: fuse::ReplyOpen) {
    //     reply.opened(fh: u64, flags: u32)
    //     debug!("Opening {:?} Inode {}", _req, _ino);

    //     reply.error(ENOSYS);
    // }
    fn readdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        mut reply: fuse::ReplyDirectory,
    ) {
        let dir = match _ino {
            1 => self.meta.get_root(),
            _ => self.meta.get_dir(_ino),
        };

        let dir = match dir {
            Ok(dir) => dir,
            Err(err) => {
                reply.error(ENOENT);
                return;
            }
        };

        for (index, entry) in dir.entries.iter().enumerate() {
            reply.add(
                1, //TODO: use real inode value here.
                index as i64,
                fuse::FileType::Directory,
                OsStr::new(&entry.name),
            );
        }
        reply.ok();
        //reply.add(ino: u64, offset: i64, kind: FileType, name: T)
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

        let dir = match _parent {
            1 => self.meta.get_root(),
            _ => self.meta.get_dir(_parent),
        };

        let dir = match dir {
            Ok(dir) => dir,
            Err(err) => {
                reply.error(ENOENT);
                return;
            }
        };

        // scan entries for the name
        for entry in dir.entries {
            if entry.name != name {
                continue;
            }

            self.lookup_entry(entry, reply);
            return;
        }

        reply.error(ENOSYS);
    }

    /// Get file attributes.
    fn getattr(&mut self, _req: &Request, _ino: u64, reply: fuse::ReplyAttr) {
        let node = match _ino {
            1 => self.meta.get_root(),
            _ => {
                reply.error(ENOENT);
                return;
            }
        };

        let node = match node {
            Ok(node) => node,
            Err(err) => {
                debug!("error getting root directory {}", err);
                reply.error(ENOENT);
                return;
            }
        };

        reply.attr(&self.ttl, &node.attr());
    }
}
