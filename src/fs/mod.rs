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

impl<'a> fuse::Filesystem for Filesystem<'a> {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        info!("Initializing file system");
        Ok(())
    }

    fn opendir(&mut self, _req: &fuse::Request, _ino: u64, _flags: u32, reply: fuse::ReplyOpen) {
        debug!("Opening {:?} Inode {}", _req, _ino);

        reply.error(ENOSYS);
    }

    /// Look up a directory entry by name and get its attributes.
    fn lookup(&mut self, _req: &Request, _parent: u64, _name: &OsStr, reply: fuse::ReplyEntry) {
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
