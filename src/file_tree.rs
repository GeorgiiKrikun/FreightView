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
    // This is a path of the extracted docker tarball; It contains the pathes with .wh. inside
    disk_rel_path: PathBuf,
    // This is a path that is visible to the user; The .wh. files are stripped
    vis_rel_path: PathBuf,
    size: u64,
}

fn parse_name(name: &str) -> (String, FileOp) {
    let stripped_name = name.strip_prefix(".wh.");
    match stripped_name {
        Some(stripped_name) => (stripped_name.to_string(), FileOp::Remove),
        None => (name.to_string(), FileOp::Add),
    }
}

#[cfg(unix)]
impl FileTreeNode {
    fn from(relpath: &Path, metadata: &Metadata) -> Result<FileTreeNode, ImageParcingError> {
        let ftype = DDiveFileType::from_ftype(metadata.file_type());
        let permissions = perm_str_from_u32(metadata.permissions().mode());
        let size = metadata.len();

        let name_to_parse = relpath.file_name();
        let parent_dir = relpath.parent();
        let name = match name_to_parse {
            Some(name) => name.to_string_lossy().to_string(),
            None => "/".to_string(),
        };

        let (name, fop) = parse_name(&name);
        let vis_rel_path = match parent_dir {
            Some(parent_dir) => {
                let mut vis_rel_path = parent_dir.to_path_buf();
                vis_rel_path.push(&name);
                vis_rel_path
            }
            None => PathBuf::from("/"),
        };

        let node = FileTreeNode {
            name,
            ftype,
            fop,
            children: RefCell::new(Vec::<Rc<RefCell<FileTreeNode>>>::new()),
            permissions,
            disk_rel_path: relpath.to_path_buf(),
            vis_rel_path,
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
            // println!("Path: {:?}", path);
            let node_rel_path = node.borrow().disk_rel_path.clone();
            // remove the leading slash
            let node_rel_path = node_rel_path
                .strip_prefix("/")
                .unwrap_or(&node_rel_path)
                .to_path_buf();
            // println!("Node rel path: {:?}", node_rel_path);

            let path_to_node = path.join(node_rel_path);
            // println!("Directory: {:?}", path_to_node);
            // println!("Contents:");

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

                    // remove the bad files
                    let mut cleaned_entries: Vec<PathBuf> = Vec::new();
                    for entry in entries {
                        match entry {
                            Ok(res) => {
                                cleaned_entries.push(res.path());
                            }
                            Err(e) => {
                                println!("Erroroneous dir entry: {}", e);
                            }
                        }
                    }
                    // sort cleaned entries; no real reason to do that, other than to have a
                    // consistent order
                    cleaned_entries.sort();

                    for entry in cleaned_entries {
                        // println!("Entry: {:?}", entry);
                        let entry_rel_path = PathBuf::from("/")
                            .join(entry.strip_prefix(path).unwrap_or(&entry).to_path_buf());
                        // println!("Entry rel path: {:?}", entry_rel_path);
                        let metadata = entry.metadata()?;
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
                }
            }
        }

        let tree = FileTree {
            parent_node,
            path_to_parent_node: path.to_path_buf(),
        };

        return Ok(tree);
    }

    fn root(&self) -> Rc<RefCell<FileTreeNode>> {
        self.parent_node.clone()
    }

    fn iter(&self) -> BreadthFirstIterator {
        let root = self.root();
        BreadthFirstIterator::new(root)
    }
}

// Define the iterator struct for breadth-first traversal
struct BreadthFirstIterator {
    queue: VecDeque<Rc<RefCell<FileTreeNode>>>,
}

impl BreadthFirstIterator {
    fn new(root_node: Rc<RefCell<FileTreeNode>>) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(root_node);
        BreadthFirstIterator { queue }
    }
}

impl Iterator for BreadthFirstIterator {
    type Item = Rc<RefCell<FileTreeNode>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.queue.pop_front() {
            let node_ref = node.borrow();
            let children_vec = node_ref.children.borrow();
            for child in children_vec.iter() {
                self.queue.push_back(child.clone());
            }
            Some(node.clone())
        } else {
            None
        }
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
    fn tree_test() {
        let tree = construct_tree();
        for nodes in tree.iter() {
            let node = nodes.borrow();
            println!(
                "Name: {}, path: {}, vis_path {}, op: {}",
                node.name,
                node.disk_rel_path.to_str().unwrap(),
                node.vis_rel_path.to_str().unwrap(),
                node.fop
            );
            if node.name == "subtest3" {
                assert_eq!(
                    node.ftype,
                    crate::docker_file_tree::DDiveFileType::Directory
                );
                assert_eq!(node.fop, crate::docker_file_tree::FileOp::Remove);
                assert_eq!(node.disk_rel_path.to_str().unwrap(), "/.wh.subtest3");
                assert_eq!(node.vis_rel_path.to_str().unwrap(), "/subtest3");
            }
            // Name: subsubtest, path: /subtest/subsubtest, vis_path /subtest/subsubtest, op: Add
            if node.name == "subsubtest" {
                assert_eq!(
                    node.ftype,
                    crate::docker_file_tree::DDiveFileType::Directory
                );
                assert_eq!(node.fop, crate::docker_file_tree::FileOp::Add);
                assert_eq!(node.disk_rel_path.to_str().unwrap(), "/subtest/subsubtest");
                assert_eq!(node.vis_rel_path.to_str().unwrap(), "/subtest/subsubtest");
            }
            // Name: subtestfile, path: /subtest/subtestfile, vis_path /subtest/subtestfile, op: Add
            if node.name == "subtestfile" {
                assert_eq!(node.ftype, crate::docker_file_tree::DDiveFileType::File);
                assert_eq!(node.fop, crate::docker_file_tree::FileOp::Add);
                assert_eq!(node.disk_rel_path.to_str().unwrap(), "/subtest/subtestfile");
                assert_eq!(node.vis_rel_path.to_str().unwrap(), "/subtest/subtestfile");
            }
            // Name: whatever, path: /subtest2/.wh.whatever, vis_path /subtest2/whatever, op: Remove
            if node.name == "whatever" {
                assert_eq!(node.ftype, crate::docker_file_tree::DDiveFileType::File);
                assert_eq!(node.fop, crate::docker_file_tree::FileOp::Remove);
                assert_eq!(
                    node.disk_rel_path.to_str().unwrap(),
                    "/subtest2/.wh.whatever"
                );
                assert_eq!(node.vis_rel_path.to_str().unwrap(), "/subtest2/whatever");
            }
        }
    }
}
