use crate::fungi::meta::{FileType, Inode, Result, Walk, WalkVisitor};
use std::path::Path;

pub struct InspectVisitor {
    file_count: u32,
    dir_count: u32,
    link_count: u32,
    total_size: u64,
}

impl InspectVisitor {
    pub fn new() -> Self {
        Self { 
            file_count: 0, 
            dir_count: 0, 
            link_count: 0, 
            total_size: 0 
        }
    }

    pub fn print_summary(&self, target: &str) {
        println!("Flist Inspection: {}", target);
        println!("==================");
        println!("Files: {}", self.file_count);
        println!("Directories: {}", self.dir_count);
        println!("Symlinks: {}", self.link_count);
        println!("Total size: {} bytes", self.total_size);
    }

    fn print_metadata(&self, path: &Path, node: &Inode) {
        let file_type_str = match node.mode.file_type() {
            FileType::Dir => "Directory",
            FileType::Regular => "Regular File",
            FileType::Link => "Symbolic Link",
            FileType::Block => "Block Device",
            FileType::Char => "Character Device",
            FileType::Socket => "Socket",
            FileType::FIFO => "FIFO",
            FileType::Unknown => "Unknown",
        };

        println!("Path: {}", path.display());
        println!("  Type: {}", file_type_str);
        println!("  Inode: {}", node.ino);
        println!("  Name: {}", node.name);
        println!("  Size: {} bytes", node.size);
        println!("  UID: {}", node.uid);
        println!("  GID: {}", node.gid);
        println!("  Mode: 0{:o}", node.mode.mode());
        println!("  Permissions: 0{:o}", node.mode.permissions());
        println!("  Device: {}", node.rdev);
        println!("  Created: {}", node.ctime);
        println!("  Modified: {}", node.mtime);
        
        if let Some(data) = &node.data {
            if node.mode.file_type() == FileType::Link {
                if let Ok(target) = String::from_utf8(data.clone()) {
                    println!("  Link Target: {}", target);
                }
            } else {
                println!("  Extra Data: {} bytes", data.len());
            }
        }
        println!("  ---");
    }
}

#[async_trait::async_trait]
impl WalkVisitor for InspectVisitor {
    async fn visit(&mut self, path: &Path, node: &Inode) -> Result<Walk> {
        // Print metadata for each file/directory
        self.print_metadata(path, node);
        
        match node.mode.file_type() {
            FileType::Dir => self.dir_count += 1,
            FileType::Regular => {
                self.file_count += 1;
                self.total_size += node.size;
            },
            FileType::Link => self.link_count += 1,
            _ => {}
        }
        Ok(Walk::Continue)
    }
}