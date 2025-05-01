use crate::{
    docker_file_tree::{DDiveFileType, FileOp},
    exceptions::ImageParcingError,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::Metadata;
// use std::fs::Permissions;
#[cfg(unix)]
use std::os::unix::fs::{/* MetadataExt,  */ PermissionsExt};
use std::path::{Path, PathBuf};
use std::rc::Rc;

struct FileTreeNode {
    name: String,
    ftype: DDiveFileType,
    fop: FileOp,
    // Children might need to be modified after creation, this is why we use RefCell
    // Multiple reference might be needed when e.g going through the tree breadth or depth first
    // Hence we use Rc<RefCell<T>> to allow multiple ownership and mutability
    children: RefCell<Vec<Rc<RefCell<FileTreeNode>>>>,
    permissions: String,
    relpath: PathBuf,
    size: u64,
}

#[cfg(unix)]
impl FileTreeNode {
    fn from(relpath: &Path, metadata: &Metadata) -> Result<FileTreeNode, ImageParcingError> {
        let ftype = DDiveFileType::from_ftype(metadata.file_type());
        let permissions = perm_str_from_u32(metadata.permissions().mode());
        let size = metadata.len();

        let name_to_parse = relpath.file_name();
        let name = match name_to_parse {
            Some(name) => name.to_string_lossy().to_string(),
            None => "/".to_string(),
        };

        let node = FileTreeNode {
            name,
            ftype,
            fop: FileOp::Add,
            children: RefCell::new(Vec::<Rc<RefCell<FileTreeNode>>>::new()),
            permissions,
            relpath: relpath.to_path_buf(),
            size,
        };

        return Ok(node);
    }
}

struct FileTree {
    parent_node: Rc<RefCell<FileTreeNode>>,
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
        let metadata = path.metadata()?;
        let relpath = PathBuf::from("/");
        let parent_node = Rc::new(RefCell::new(FileTreeNode::from(&relpath, &metadata)?));

        let mut queue = VecDeque::<Rc<RefCell<FileTreeNode>>>::new();
        queue.push_front(parent_node.clone());

        while !queue.is_empty() {
            let node = queue
                .pop_front()
                .expect("Queue should not be empty at this point, aborting");
            println!("Path: {:?}", path);
            let node_rel_path = node.borrow().relpath.clone();
            // remove the leading slash
            let node_rel_path = node_rel_path
                .strip_prefix("/")
                .unwrap_or(&node_rel_path)
                .to_path_buf();
            println!("Node rel path: {:?}", node_rel_path);

            let path_to_node = path.join(node_rel_path);
            println!("Directory: {:?}", path_to_node);
            println!("Contents:");

            let metadata = path_to_node.metadata()?;
            let ftype = DDiveFileType::from_ftype(metadata.file_type());
            match &ftype {
                DDiveFileType::Badfile => {
                    // Do nothing, skip bad files
                }
                DDiveFileType::Symlink => {
                    // Do nothing, skip bad files
                }
                DDiveFileType::File => {
                    // Do nothing, skip bad files
                }
                DDiveFileType::Directory => {
                    let entries = std::fs::read_dir(path_to_node.as_path())?;
                    for entry in entries {
                        match entry {
                            Ok(pentry) => {
                                println!("Entry: {:?}", pentry);
                                let entry_path = pentry.path();
                                let entry_rel_path = PathBuf::from("/").join(
                                    entry_path
                                        .strip_prefix(path)
                                        .unwrap_or(&entry_path)
                                        .to_path_buf(),
                                );
                                println!("Entry rel path: {:?}", entry_rel_path);
                                let metadata = entry_path.metadata()?;
                                let child_node = Rc::new(RefCell::new(FileTreeNode::from(
                                    &entry_rel_path,
                                    &metadata,
                                )?));
                                node.borrow_mut()
                                    .children
                                    .borrow_mut()
                                    .push(child_node.clone());
                                queue.push_back(child_node.clone());
                            }
                            Err(e) => {
                                println!("Erroroneous dir entry: {}", e);
                            }
                        }
                    }
                }
            }
        }

        let tree = FileTree {
            parent_node,
            path_to_parent_node: path.to_path_buf(),
        };

        return Ok(tree);
    }
}

#[cfg(test)]
mod tests {
    use super::FileTree;
    use std::path::PathBuf;

    fn construct_tree() -> FileTree {
        let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project_dir.join("test-files");
        FileTree::new(&test_dir).unwrap()
    }

    #[test]
    fn new_tree_test() {
        let tree = construct_tree();
    }
}
