use super::inode::Inode;
use super::Manager;
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
    pub inode: Inode,
    //pub parent: Inode,
    pub name: String,
    pub size: u64,
    pub acl: String,
    pub modification: u32,
    pub creation: u32,
    pub kind: EntryKind,
}

#[derive(Debug)]
pub struct Dir {
    pub inode: Inode,
    pub parent: Inode,
    pub name: String,
    pub location: String,
    pub size: u64,
    pub acl: String,
    pub modification: u32,
    pub creation: u32,
    pub entries: Vec<Entry>,
}

impl Dir {
    fn entries(
        manager: &Manager,
        inode: Inode,
        dir: &schema_capnp::dir::Reader,
    ) -> Result<Vec<Entry>, Error> {
        use schema_capnp::inode::attributes::Which;

        let mut entries: Vec<Entry> = vec![];
        let mut x = 0;

        for entry in dir.get_contents()? {
            x += 1;
            let mut entry_inode = inode.at(x);
            let attrs = entry.get_attributes();
            let kind = match attrs.which()? {
                Which::Dir(d) => {
                    let key = String::from(d?.get_key()?);
                    entry_inode = manager.dir_inode_from_key(&key).unwrap_or(entry_inode);
                    EntryKind::Dir(DirEntry { key: key })
                }
                _ => EntryKind::Unknown,
            };

            if let EntryKind::Unknown = kind {
                continue;
            }

            let e = Entry {
                inode: entry_inode,
                //parent: inode,
                name: String::from(entry.get_name()?),
                size: entry.get_size(),
                acl: String::from(entry.get_aclkey()?),
                modification: entry.get_modification_time(),
                creation: entry.get_creation_time(),
                kind: kind,
            };
            entries.push(e);
        }

        Ok(entries)
    }

    pub fn from(
        manager: &Manager,
        inode: Inode,
        dir: schema_capnp::dir::Reader,
    ) -> Result<Dir, Error> {
        let entries = Self::entries(manager, inode, &dir)?;

        Ok(Dir {
            inode: inode,
            name: String::from(dir.get_name()?),
            location: String::from(dir.get_location()?),
            parent: match inode.ino() {
                1 => inode,
                _ => match dir.get_parent() {
                    Ok(v) => manager.dir_inode_from_key(&v).unwrap_or(inode),
                    Err(_) => inode,
                },
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
            ino: self.inode.ino(),
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
