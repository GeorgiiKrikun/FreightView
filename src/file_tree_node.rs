use crate::docker_file_tree::{DDiveFileType, FileOp};

pub struct FileTreeNode {
    name: String,
    ftype: DDiveFileType,
    fop: FileOp,
    children: Vec<FileTreeNode>,
    permissions: String,
    size: String,
}
