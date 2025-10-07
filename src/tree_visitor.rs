use crate::fungi::meta::{FileType, Inode, Result, Walk, WalkVisitor};
use std::path::Path;

pub struct TreeVisitor {
    // We don't need to track depth since the path already contains the structure
}

impl Default for TreeVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeVisitor {
    pub fn new() -> Self {
        Self {}
    }

    fn print_entry(&self, path: &Path, node: &Inode) {
        // Calculate depth from the path
        let depth = path.components().count().saturating_sub(1);
        let indent = "  ".repeat(depth);
        let file_type = match node.mode.file_type() {
            FileType::Dir => "📁",
            FileType::Regular => "📄",
            FileType::Link => "🔗",
            _ => "❓",
        };

        // Get just the filename
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_else(|| path.to_string_lossy());

        println!("{}{} {}", indent, file_type, name);
    }
}

#[async_trait::async_trait]
impl WalkVisitor for TreeVisitor {
    async fn visit(&mut self, path: &Path, node: &Inode) -> Result<Walk> {
        self.print_entry(path, node);
        Ok(Walk::Continue)
    }
}
