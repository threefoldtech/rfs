use super::inode::Inode;
use super::Manager;
use crate::schema_capnp;
use capnp::Error;
use capnp::{message, serialize};
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
pub struct FileBlock {
    pub Hash: Vec<u8>,
    pub Key: Vec<u8>,
}

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
    fn kind(self: Box<Self>) -> EntryKind {
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

pub struct Dir {
    pub inode: Inode,
    msg: message::Reader<serialize::OwnedSegments>,
}

impl Dir {
    pub fn new(inode: Inode, data: Vec<u8>) -> Result<Dir, Error> {
        let mut raw: &[u8] = data.as_ref();

        let msg = serialize::read_message(&mut raw, message::ReaderOptions::default())?;

        Ok(Dir {
            inode: inode,
            msg: msg,
        })
    }

    pub fn parent(&self, manager: &Manager) -> Inode {
        let reader = self.msg.get_root::<schema_capnp::dir::Reader>().unwrap();

        match self.inode.ino() {
            1 => self.inode,
            _ => match reader.get_parent() {
                Ok(v) => manager.dir_inode_from_key(&v).unwrap_or(self.inode),
                Err(_) => self.inode,
            },
        }
    }

    pub fn name(&self) -> &str {
        self.msg
            .get_root::<schema_capnp::dir::Reader>()
            .unwrap()
            .get_name()
            .unwrap()
    }

    pub fn location(&self) -> &str {
        self.msg
            .get_root::<schema_capnp::dir::Reader>()
            .unwrap()
            .get_location()
            .unwrap()
    }

    pub fn size(&self) -> u64 {
        self.msg
            .get_root::<schema_capnp::dir::Reader>()
            .unwrap()
            .get_size()
    }

    pub fn modification(&self) -> u32 {
        self.msg
            .get_root::<schema_capnp::dir::Reader>()
            .unwrap()
            .get_modification_time()
    }

    pub fn creation(&self) -> u32 {
        self.msg
            .get_root::<schema_capnp::dir::Reader>()
            .unwrap()
            .get_creation_time()
    }

    pub fn entries(&self, manager: &Manager) -> Result<Vec<Entry>, Error> {
        /*
        This definitely needs refactoring
        */
        use schema_capnp::inode::attributes::Which;

        let dir = self.msg.get_root::<schema_capnp::dir::Reader>()?;

        let mut entries: Vec<Entry> = vec![];
        let mut x = 0;

        for entry in dir.get_contents()? {
            x += 1;
            let mut entry_inode = self.inode.at(x);
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
                        blocks: match f.get_blocks() {
                            Ok(blocks) => {
                                let mut result = vec![];
                                for block in blocks {
                                    result.push(FileBlock {
                                        Hash: Vec::from(block.get_hash()?),
                                        Key: Vec::from(block.get_key()?),
                                    });
                                }
                                result
                            }
                            Err(err) => return Err(err),
                        },
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

            entries.push(e);
        }

        Ok(entries)
    }
}

impl Node for Dir {
    fn kind(self: Box<Self>) -> EntryKind {
        EntryKind::Dir(DirEntry { key: String::new() })
    }

    fn node_type(&self) -> fuse::FileType {
        fuse::FileType::Directory
    }

    fn attr(&self) -> fuse::FileAttr {
        let mtime = self.modification();
        let ctime = self.creation();

        fuse::FileAttr {
            ino: self.inode.ino(),
            size: self.size(),
            blocks: 0,
            atime: Timespec::new(mtime as i64, 0),
            mtime: Timespec::new(mtime as i64, 0),
            ctime: Timespec::new(ctime as i64, 0),
            crtime: Timespec::new(ctime as i64, 0),
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
