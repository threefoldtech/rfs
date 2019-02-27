#[derive(Debug)]
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
