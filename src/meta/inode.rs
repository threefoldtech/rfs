use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct Mask(u8);

impl Mask {
    /// get i node mask
    pub fn from(hold: u64) -> Mask {
        let mut hold = hold;
        let mut shift: u8 = 0;
        while hold != 0 {
            shift += 1;
            hold = hold >> shift * 8;
        }

        Mask(shift)
    }

    /// split an inode into (dir, index)
    pub fn split(&self, i: u64) -> (u64, u64) {
        let index: u64 = i >> self.0 * 8;
        let shift = (8 - self.0) * 8;
        let dir: u64 = (i << shift) >> shift;

        (dir, index)
    }

    pub fn merge(&self, dir: u64, index: u64) -> u64 {
        index << self.0 * 8 | dir
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Inode(Mask, u64);

impl fmt::Display for Inode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:016x}", self.1)
    }
}

impl Inode {
    pub fn new(mask: Mask, ino: u64) -> Inode {
        Inode(mask, ino)
    }

    pub fn mask(&self) -> Mask {
        self.0
    }

    pub fn ino(&self) -> u64 {
        self.1
    }

    /// split this value into (dir, index)
    pub fn split(&self) -> (u64, u64) {
        self.0.split(self.1)
    }

    /// dir inode of this inode (parent).
    /// Same value in case index part is 0
    pub fn dir(&self) -> Inode {
        let (dir, _) = self.split();
        Inode::new(self.0, dir)
    }

    /// index of inode, 0 means the directory entry, all sub entries start with 1
    pub fn index(&self) -> u64 {
        let (_, index) = self.split();
        index
    }

    /// gets the inode value of an entry under this inode directory
    pub fn at(&self, index: u64) -> Inode {
        let value = self.0.merge(self.dir().ino(), index);
        Self::new(self.0, value)
    }
}
