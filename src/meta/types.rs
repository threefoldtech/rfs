use super::inode::Inode;
use super::Manager;
use crate::schema_capnp;
use anyhow::Result;
use capnp::{message, serialize};
use time::Timespec;

const BLOCK_SIZE: u64 = 4 * 1024;

pub trait Node {
    fn attr(&self) -> fuse::FileAttr;
    fn node_type(&self) -> fuse::FileType;
    fn kind(&self) -> EntryKind;
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub key: String,
}
#[derive(Debug, Clone)]
pub struct FileBlock {
    pub hash: Vec<u8>,
    pub key: Vec<u8>,
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
    fn kind(&self) -> EntryKind {
        self.kind.clone()
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
            blocks: self.size / BLOCK_SIZE + if self.size % BLOCK_SIZE > 0 { 1 } else { 0 },
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

#[derive(Debug, Clone)]
pub struct Dir {
    pub inode: Inode,
    pub name: String,
    pub location: String,
    pub size: u64,
    pub modification_time: u32,
    pub creation_time: u32,
    pub parent: Inode,
    pub entries: Vec<Entry>,
}

impl Dir {
    pub fn new(mgr: &Manager, data: &Vec<u8>, inode: Inode) -> Result<Dir> {
        let mut raw: &[u8] = data.as_ref();

        let msg = serialize::read_message(&mut raw, message::ReaderOptions::default())?;

        let root = msg.get_root::<schema_capnp::dir::Reader>()?;
        let name: String = root.get_name()?.into();
        let location: String = root.get_location()?.into();
        let size = root.get_size();
        let modification_time = root.get_modification_time();
        let creation_time = root.get_creation_time();
        let entries = Dir::entries(inode, root, mgr)?;
        let parent = Dir::parent(inode, root, mgr)?;

        Ok(Dir {
            inode,
            name,
            location,
            size,
            modification_time,
            creation_time,
            parent,
            entries,
        })
    }

    fn parent(inode: Inode, dir: schema_capnp::dir::Reader, manager: &Manager) -> Result<Inode> {
        match inode.ino() {
            1 => Ok(inode),
            _ => {
                let key = dir.get_parent()?;
                manager.dir_inode_from_key(&key)
            }
        }
    }

    fn entries(
        ino: Inode,
        dir: schema_capnp::dir::Reader,
        manager: &Manager,
    ) -> Result<Vec<Entry>> {
        /*
        This definitely needs refactoring
        */
        use schema_capnp::inode::attributes::Which;

        let mut entries: Vec<Entry> = vec![];
        let mut x = 0;

        for entry in dir.get_contents()? {
            x += 1;
            let mut entry_inode = ino.at(x);
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
                                        hash: Vec::from(block.get_hash()?),
                                        key: Vec::from(block.get_key()?),
                                    });
                                }
                                result
                            }
                            Err(err) => return Err(anyhow!(err)),
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
    fn kind(&self) -> EntryKind {
        EntryKind::Dir(DirEntry { key: String::new() })
    }

    fn node_type(&self) -> fuse::FileType {
        fuse::FileType::Directory
    }

    fn attr(&self) -> fuse::FileAttr {
        let mtime = self.modification_time as i64;
        let ctime = self.creation_time as i64;

        fuse::FileAttr {
            ino: self.inode.ino(),
            size: self.size,
            blocks: 0,
            atime: Timespec::new(mtime, 0),
            mtime: Timespec::new(mtime, 0),
            ctime: Timespec::new(ctime, 0),
            crtime: Timespec::new(ctime, 0),
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

pub enum Either {
    Entry(Entry),
    Dir(Dir),
}

impl Either {
    pub fn dir(dir: Dir) -> Either {
        Either::Dir(dir)
    }

    pub fn entry(entry: Entry) -> Either {
        Either::Entry(entry)
    }
}

impl Node for Either {
    fn kind(&self) -> EntryKind {
        match self {
            Either::Entry(ref entry) => entry.kind(),
            Either::Dir(ref dir) => dir.kind(),
        }
    }

    fn node_type(&self) -> fuse::FileType {
        match self {
            Either::Entry(ref entry) => entry.node_type(),
            Either::Dir(ref dir) => dir.node_type(),
        }
    }

    fn attr(&self) -> fuse::FileAttr {
        match self {
            Either::Entry(ref entry) => entry.attr(),
            Either::Dir(ref dir) => dir.attr(),
        }
    }
}
