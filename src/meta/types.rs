use crate::schema_capnp;
use capnp::Error;
use std::time::Instant;
use time::Timespec;

/*
    name    @0: Text;
    size    @1: UInt64;           # in bytes

    attributes: union {
        dir     @2: SubDir;
        file    @3: File;
        link    @4: Link;
        special @5: Special;
    }

    aclkey           @6: Text;    # is pointer to ACL # FIXME: need to be int
    modificationTime @7: UInt32;
    creationTime     @8: UInt32;
*/
#[derive(Debug)]
pub struct DirEntry {
    pub key: String,
}

#[derive(Debug)]
pub enum EntryKind {
    Unknown,
    Dir(DirEntry),
}

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub acl: String,
    pub modification: u32,
    pub creation: u32,
    pub kind: EntryKind,
}

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
    pub entries: Vec<Entry>,
}

impl Dir {
    pub fn from(inode: u64, dir: &schema_capnp::dir::Reader) -> Result<Dir, Error> {
        let mut entries: Vec<Entry> = vec![];

        use schema_capnp::inode::attributes::Which;
        for entry in dir.get_contents()? {
            let attrs = entry.get_attributes();
            let kind = match attrs.which()? {
                Which::Dir(d) => EntryKind::Dir(DirEntry {
                    key: String::from(d?.get_key()?),
                }),
                _ => EntryKind::Unknown,
            };

            let e = Entry {
                name: String::from(entry.get_name()?),
                size: entry.get_size(),
                acl: String::from(entry.get_aclkey()?),
                modification: entry.get_modification_time(),
                creation: entry.get_creation_time(),
                kind: kind,
            };
            entries.push(e);
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
            entries: entries,
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
