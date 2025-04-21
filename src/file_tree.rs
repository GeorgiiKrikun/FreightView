use crate::{
    docker_file_tree::{DDiveFileType, FileOp},
    exceptions::ImageParcingError,
};
use std::fs::Permissions;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

struct FileTreeNode {
    name: String,
    ftype: DDiveFileType,
    fop: FileOp,
    children: Vec<FileTreeNode>,
    permissions: String,
    size: u64,
}

struct FileTree {
    parent_node: FileTreeNode,
    path_to_parent_node: PathBuf,
}

#[cfg(unix)]
fn perm_str_from_u32(perm: u32) -> String {
    let mut str = String::new();
    if perm & 0o400 != 0 {
        str.push('r');
    } else {
        str.push('-');
    }
    if perm & 0o200 != 0 {
        str.push('w');
    } else {
        str.push('-');
    }
    if perm & 0o100 != 0 {
        str.push('x');
    } else {
        str.push('-');
    }
    if perm & 0o040 != 0 {
        str.push('r');
    } else {
        str.push('-');
    }
    if perm & 0o020 != 0 {
        str.push('w');
    } else {
        str.push('-');
    }
    if perm & 0o010 != 0 {
        str.push('x');
    } else {
        str.push('-');
    }
    if perm & 0o004 != 0 {
        str.push('r');
    } else {
        str.push('-');
    }
    if perm & 0o002 != 0 {
        str.push('w');
    } else {
        str.push('-');
    }
    if perm & 0o001 != 0 {
        str.push('x');
    } else {
        str.push('-');
    }
    return str;
}

impl FileTree {
    fn new(path: &Path) -> Result<FileTree, ImageParcingError> {
        let metadata = std::fs::metadata(path)?;
        let ftype = DDiveFileType::from_ftype(metadata.file_type());
        let perm = metadata.permissions();
        if ftype != DDiveFileType::Directory {
            return Err(ImageParcingError::LayerParsingError);
        }
        let parent_node = FileTreeNode {
            name: String::from("/"),
            ftype,
            fop: FileOp::Add,
            children: Vec::new(),
            permissions: perm_str_from_u32(perm.mode()),
            size: metadata.len(),
        };
        let tree = FileTree {
            parent_node,
            path_to_parent_node: path.to_path_buf(),
        };
        return Ok(tree);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    //    fn construct_tree() -> super::TreeNode {
    //        let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    //        let test_dir = project_dir.join("test-files");
    //        let mut parent = super::TreeNode::new(
    //            &super::DDiveFileType::Directory,
    //            &super::FileOp::Add,
    //            &PathBuf::from(""),
    //        );
    //        parse_directory_into_tree(&test_dir, test_dir.clone(), &mut parent);
    //        let parent = parent.prettyfy();
    //        return parent;
    //    }
    //
    //    #[test]
    //    fn test_print_tree() {
    //        let tree = construct_tree();
    //        let mut out = String::new();
    //        tree.print_tree(0, &mut out);
    //        println!("{}", out);
    //    }
}
