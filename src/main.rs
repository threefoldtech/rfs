extern crate fuse;

mod fs;

use std::path;
use std::ffi;

fn main() {
    let p = path::Path::new("/tmp/mnt");
    let o: [&ffi::OsStr; 0] = [];

    let f = fs::FS{};
    fuse::mount(f, &p, &o);
}
