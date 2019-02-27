use crate::schema_capnp;
use capnp::Error;
use std::time::Instant;
use time::Timespec;

#[derive(Debug)]
pub struct Dir {
    pub inode: u64,
    pub name: String,
    pub location: String,
    pub parent: String,
    pub size: u64,
    pub acl: String,
    pub modification: u32,
    pub creation: u32,
}

impl Dir {
    pub fn from(inode: u64, dir: &schema_capnp::dir::Reader) -> Result<Dir, Error> {
        for entry in dir.get_contents()? {
            println!("{:?}", entry.get_name()?);
        }

        Ok(Dir {
            inode: inode,
            name: String::from(dir.get_name()?),
            location: String::from(dir.get_location()?),
            parent: match dir.has_parent() {
                true => String::from(dir.get_parent()?),
                _ => String::new(),
            },
            size: dir.get_size(),
            acl: String::from(dir.get_aclkey()?),
            modification: dir.get_modification_time(),
            creation: dir.get_creation_time(),
        })
    }

    pub fn attr(&self) -> fuse::FileAttr {
        fuse::FileAttr {
            ino: self.inode,
            size: self.size,
            blocks: 0,
            atime: Timespec::new(self.modification as i64, 0),
            mtime: Timespec::new(self.modification as i64, 0),
            ctime: Timespec::new(self.creation as i64, 0),
            crtime: Timespec::new(self.creation as i64, 0),
            kind: fuse::FileType::Directory,
            perm: 0o0755,
            nlink: 1,
            uid: 1000,
            gid: 0,
            rdev: 0,
            flags: 0,
        }
    }
}
