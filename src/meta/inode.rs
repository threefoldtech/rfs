use std::fmt;

#[derive(Debug, Clone, Copy, Hash, Eq)]
pub struct Mask(u8);

impl std::cmp::PartialEq for Mask {
    fn eq(&self, other: &Mask) -> bool {
        self.0 == other.0
    }
}

impl Mask {
    /// get i node mask
    pub fn from(max: u64) -> Mask {
        let mut hold = max;
        let mut width: u8 = 0;
        while hold != 0 {
            width += 1;
            hold = hold >> 8;
        }
        // width is how many bytes can hold the max
        // number of directories
        Mask(width)
    }

    /// split an inode into (dir, index)
    pub fn split(&self, i: u64) -> (u64, u64) {
        let index: u64 = i >> self.0 * 8;
        let shift = (std::mem::size_of::<u64>() - self.0 as usize) * 8;
        let dir: u64 = (i << shift) >> shift;

        (dir, index)
    }

    pub fn merge(&self, dir: u64, index: u64) -> u64 {
        // to build an id of the dir+entry we now the width of the
        // mask, right? so we can shift the index to the lift
        // to make a free space at the right to the directory id

        index << self.0 * 8 | dir
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq)]
pub struct Inode(Mask, u64);

impl fmt::Display for Inode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:016x}", self.1)
    }
}

impl std::cmp::PartialEq for Inode {
    fn eq(&self, other: &Inode) -> bool {
        self.1 == other.1
    }
}

impl Inode {
    pub fn new(mask: Mask, ino: u64) -> Inode {
        Inode(mask, ino)
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

    /// gets the inode value of an entry under this inode directory
    pub fn at(&self, index: u64) -> Inode {
        let value = self.0.merge(self.dir().ino(), index);
        Self::new(self.0, value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn mask() {
        let mask = Mask::from(0xff);
        assert_eq!(mask.0, 1);
        let inode = mask.merge(0xf1, 1000);
        let (dir, index) = mask.split(inode);
        assert_eq!(0xf1, dir);
        assert_eq!(1000, index);
    }

    #[test]
    fn mask_big() {
        let mask = Mask::from(0xffff);
        assert_eq!(2, mask.0);
        let inode = mask.merge(0xabcd, 0x1234);
        let (dir, index) = mask.split(inode);
        assert_eq!(0xabcd, dir);
        assert_eq!(0x1234, index);
    }
}
