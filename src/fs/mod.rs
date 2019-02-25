use libc::{c_int, ENOSYS};

pub struct Filesystem {}

impl Filesystem {
    /// creates a new instance of the filesystem
    pub fn new() -> Filesystem {
        Filesystem {}
    }
}

impl fuse::Filesystem for Filesystem {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        info!("Initializing file system");
        Ok(())
    }

    fn opendir(&mut self, _req: &fuse::Request, _ino: u64, _flags: u32, reply: fuse::ReplyOpen) {
        debug!("Opening {:?} Inode {}", _req, _ino);

        reply.error(ENOSYS);
    }

    /// Get file attributes.
    fn getattr(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyAttr) {
        // reply.attr(ttl: &Timespec, attr: &FileAttr)
        reply.error(ENOSYS);
    }
}
