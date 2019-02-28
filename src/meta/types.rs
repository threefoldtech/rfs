use super::inode::Inode;
use super::Manager;
use crate::schema_capnp;
use capnp::Error;
use std::time::Instant;
use time::Timespec;

const BlockSize: u64 = 4 * 1024;

pub trait Node {
    fn attr(&self) -> fuse::FileAttr;
    fn node_type(&self) -> fuse::FileType;
    fn kind(self: Box<Self>) -> EntryKind;
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub key: String,
}
#[derive(Debug, Clone)]
pub struct FileBlock {}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub block_size: u16,
    pub blocks: Vec<FileBlock>,
}

#[derive(Debug, Clone)]
pub struct LinkEntry {
    pub target: String,
}

#[derive(Debug, Clone)]
pub enum EntryKind {
    Unknown,
    Dir(DirEntry),
    File(FileEntry),
    Link(LinkEntry),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub inode: Inode,
    pub name: String,
    pub size: u64,
    pub acl: String,
    pub modification: u32,
    pub creation: u32,
    pub kind: EntryKind,
}

impl Node for Entry {
    fn kind(self: Box<Entry>) -> EntryKind {
        self.kind
    }

    fn node_type(&self) -> fuse::FileType {
        use fuse::FileType;

        match self.kind {
            EntryKind::File(_) => FileType::RegularFile,
            EntryKind::Dir(_) => FileType::Directory,
            EntryKind::Link(_) => FileType::Symlink,
            _ => FileType::Socket, // this should never happen
        }
    }

    fn attr(&self) -> fuse::FileAttr {
        fuse::FileAttr {
            ino: self.inode.ino(),
            size: if let EntryKind::Link(l) = &self.kind {
                l.target.len() as u64
            } else {
                self.size
            },
            blocks: self.size / BlockSize + if self.size % BlockSize > 0 { 1 } else { 0 },
            atime: Timespec::new(self.modification as i64, 0),
            mtime: Timespec::new(self.modification as i64, 0),
            ctime: Timespec::new(self.creation as i64, 0),
            crtime: Timespec::new(self.creation as i64, 0),
            kind: self.node_type(),
            perm: 0o0755,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        }
    }
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
                Which::File(f) => {
                    let f = f?;
                    EntryKind::File(FileEntry {
                        block_size: f.get_block_size(),
                        blocks: vec![],
                    })
                }
                Which::Link(l) => {
                    let l = l?;
                    EntryKind::Link(LinkEntry {
                        target: String::from(l.get_target()?),
                    })
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

            debug!("Dir {} entry {:?}", inode, e);
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
}

impl Node for Dir {
    fn kind(self: Box<Dir>) -> EntryKind {
        EntryKind::Dir(DirEntry { key: String::new() })
    }

    fn node_type(&self) -> fuse::FileType {
        fuse::FileType::Directory
    }

    fn attr(&self) -> fuse::FileAttr {
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
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        }
    }
}
