use crate::{
    docker_file_tree::{DDiveFileType, FileOp},
    exceptions::{GUIError, ImageParcingError},
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::Metadata;
// use std::fs::Permissions;

#[cfg(unix)]
use std::os::unix::fs::{/* MetadataExt,  */ PermissionsExt};
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct FileTreeNode {
    // Children might need to be modified after creation, this is why we use RefCell
    // Multiple reference might be needed when e.g going through the tree breadth or depth first
    // Hence we use Rc<RefCell<T>> to allow multiple ownership and mutability
    children: RefCell<Vec<Rc<RefCell<FileTreeNode>>>>,
    data: FileTreeNodeData,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Debug)]
pub struct FileTreeNodeData {
    name: String,
    ftype: DDiveFileType,
    fop: FileOp,
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

        let data = FileTreeNodeData {
            name,
            ftype,
            fop,
            permissions,
            disk_rel_path: relpath.to_path_buf(),
            vis_rel_path,
            size,
        };

        let node = FileTreeNode {
            children: RefCell::new(Vec::<Rc<RefCell<FileTreeNode>>>::new()),
            data,
        };

        return Ok(node);
    }

    fn from_data(data: &FileTreeNodeData) -> FileTreeNode {
        let node = FileTreeNode {
            children: RefCell::new(Vec::<Rc<RefCell<FileTreeNode>>>::new()),
            data: data.clone(),
        };
        return node;
    }

    fn get_child(self: &Self, i: usize) -> Option<Rc<RefCell<FileTreeNode>>> {
        match self.children.borrow().get(i) {
            Some(child) => Some(child.clone()),
            None => None,
        }
    }

    fn add_child(self: &Self, child: Rc<RefCell<FileTreeNode>>) {
        self.children.borrow_mut().push(child);
    }

    pub fn get_children_names(self: &Self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for child in self.children.borrow().iter() {
            names.push(child.borrow().data.name.clone());
        }
        return names;
    }

    pub fn get_children_paths(self: &Self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        for child in self.children.borrow().iter() {
            paths.push(child.borrow().data.vis_rel_path.clone());
        }
        return paths;
    }

    pub fn get_n_children(self: &Self) -> usize {
        return self.children.borrow().len();
    }

    pub fn name(self: &Self) -> String {
        return self.data.name.clone();
    }

    pub fn fop(self: &Self) -> FileOp {
        return self.data.fop.clone();
    }

    pub fn path(self: &Self) -> PathBuf {
        return self.data.vis_rel_path.clone();
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct FileTree {
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
    pub fn new(path: &Path) -> Result<FileTree, ImageParcingError> {
        let metadata = path.symlink_metadata()?;
        let relpath = PathBuf::from("/");
        let parent_node = Rc::new(RefCell::new(FileTreeNode::from(&relpath, &metadata)?));

        let mut queue = VecDeque::<Rc<RefCell<FileTreeNode>>>::new();
        queue.push_front(parent_node.clone());

        while !queue.is_empty() {
            let node = queue
                .pop_front()
                .expect("Queue should not be empty at this point, aborting");
            // println!("Path: {:?}", path);
            let node_rel_path = node.borrow().data.disk_rel_path.clone();
            // remove the leading slash
            let node_rel_path = node_rel_path
                .strip_prefix("/")
                .unwrap_or(&node_rel_path)
                .to_path_buf();
            // println!("Node rel path: {:?}", node_rel_path);
            let node_rel_path_str = node_rel_path.to_str().unwrap_or("");
            if node_rel_path_str == "var/lock" {
                println!("Got to var/lock, skipping");
            }
            let path_to_node = path.join(node_rel_path);
            // println!("Directory: {:?}", path_to_node);
            // println!("Contents:");
            let metadata = path_to_node.symlink_metadata()?;

            // println!("Metadata: {:?}", metadata);
            let ftype = DDiveFileType::from_ftype(metadata.file_type());
            match &ftype {
                DDiveFileType::Badfile => {
                    // println!("Bad file: {:?}", path_to_node);
                    // Do nothing, skip bad files
                }
                DDiveFileType::Symlink => {
                    // println!("Symlink: {:?}", path_to_node);
                    // Do nothing, skip bad files
                }
                DDiveFileType::File => {
                    // println!("File: {:?}", path_to_node);
                    // Do nothing, skip bad files
                }
                DDiveFileType::Directory => {
                    // println!("Directory: {:?}", path_to_node);
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
                        let metadata = entry.symlink_metadata()?;
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

    pub fn new_from_node(node: Rc<RefCell<FileTreeNode>>) -> FileTree {
        let tree = FileTree {
            parent_node: node,
            path_to_parent_node: PathBuf::from("/"),
        };
        return tree;
    }
    /// This function is used to filter the tree based on the full path
    /// It returns a new tree with the filtered nodes
    /// If the path is not filtered, it returns a full tree
    ///
    /// It always returns a tree, even if the path is not found,
    /// But in in this case, it will add GUI error to the return value
    pub fn filter_tree_full_path(&self, filter: &str) -> (FileTree, Option<GUIError>) {
        let filter: Vec<&str> = filter.split("/").collect();
        // clean up empty strings
        let filter: Vec<&str> = filter.iter().filter(|&x| x != &"").map(|x| *x).collect();
        if filter.len() == 0 {
            return (self.clone(), None);
        }

        let root = self.root();
        let root_data = &root.borrow().data;
        let new_root = Rc::new(RefCell::new(FileTreeNode::from_data(root_data)));
        let mut new_current = new_root.clone();
        let mut old_current = root.clone();

        // last search string should be taken care separately as it should not filter when the path is not
        // yet fully typed
        for d in 0..filter.len() - 1 {
            let subfilter: &str = filter[d];
            let mut next_ind: Option<usize> = None;
            let children_names = old_current.borrow().get_children_names();
            for i in 0..children_names.len() {
                if children_names[i] == subfilter {
                    next_ind = Some(i);
                    break;
                }
            }

            match next_ind {
                Some(n) => {
                    let old_child_opt = old_current.borrow().get_child(n);
                    match &old_child_opt {
                        Some(old_child) => {}
                        None => return (self.clone(), Some(GUIError::CantFilterTree)),
                    };
                    let old_child = old_child_opt.unwrap();
                    let old_child_data = old_child.borrow().data.clone();
                    let new_child = Rc::new(RefCell::new(FileTreeNode::from_data(&old_child_data)));
                    new_current.borrow().add_child(new_child.clone());
                    new_current = new_child.clone();
                    old_current = old_child.clone();
                }
                None => return (self.clone(), None),
            }
        }

        // Parse last filter
        let subfilter: &str = filter[filter.len() - 1];
        let mut inds: Vec<usize> = Vec::new();

        let children_names = old_current.borrow().get_children_names();
        for i in 0..children_names.len() {
            if children_names[i].starts_with(subfilter) {
                inds.push(i);
            }
        }

        if inds.len() == 0 {
            return (self.clone(), Some(GUIError::CantFilterTree));
        }

        let mut new_children = Vec::<Rc<RefCell<FileTreeNode>>>::new();

        for i in inds {
            // Get the orig child node
            let old_child_opt = old_current.borrow().get_child(i);
            match &old_child_opt {
                Some(old_child) => {}
                None => return (self.clone(), Some(GUIError::CantFilterTree)),
            };
            let old_child = old_child_opt.unwrap();

            let old_child_data = old_child.borrow().data.clone();
            let new_child = Rc::new(RefCell::new(FileTreeNode::from_data(&old_child_data)));
            // Last child should also have original node chidren added to keeep the tree
            new_child.borrow_mut().children = old_child.borrow_mut().children.clone();
            new_children.push(new_child.clone());
        }

        // sort children by name
        new_children.sort_by(|a, b| {
            let a_name = a.borrow().data.name.clone();
            let b_name = b.borrow().data.name.clone();
            a_name.cmp(&b_name)
        });
        new_current
            .borrow()
            .children
            .borrow_mut()
            .extend(new_children);

        let new_tree = FileTree {
            parent_node: new_root,
            path_to_parent_node: self.path_to_parent_node.clone(),
        };
        return (new_tree, None);
    }

    pub fn root(&self) -> Rc<RefCell<FileTreeNode>> {
        self.parent_node.clone()
    }

    pub fn iter(&self) -> BreadthFirstIterator {
        let root = self.root();
        BreadthFirstIterator::new(root)
    }

    fn get_node_by_name(&self, name: &str) -> Option<Rc<RefCell<FileTreeNode>>> {
        for node in self.iter() {
            if node.borrow().data.name == name {
                return Some(node.clone());
            }
        }
        None
    }
}

// Define the iterator struct for breadth-first traversal
pub struct BreadthFirstIterator {
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
    use assert_matches::assert_matches;
    use std::fs::File;
    use std::path::PathBuf;

    fn construct_tree() -> FileTree {
        let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project_dir.join("test-assets/test-files");
        FileTree::new(&test_dir).unwrap()
    }

    #[test]
    fn tree() {
        let tree = construct_tree();
        for nodes in tree.iter() {
            let node = nodes.borrow();
            if node.data.name == "subtest3" {
                assert_eq!(
                    node.data.ftype,
                    crate::docker_file_tree::DDiveFileType::Directory
                );
                assert_eq!(node.data.fop, crate::docker_file_tree::FileOp::Remove);
                assert_eq!(node.data.disk_rel_path.to_str().unwrap(), "/.wh.subtest3");
                assert_eq!(node.data.vis_rel_path.to_str().unwrap(), "/subtest3");
            }
            if node.data.name == "subsubtest" {
                assert_eq!(
                    node.data.ftype,
                    crate::docker_file_tree::DDiveFileType::Directory
                );
                assert_eq!(node.data.fop, crate::docker_file_tree::FileOp::Add);
                assert_eq!(
                    node.data.disk_rel_path.to_str().unwrap(),
                    "/subtest/subsubtest"
                );
                assert_eq!(
                    node.data.vis_rel_path.to_str().unwrap(),
                    "/subtest/subsubtest"
                );
            }
            if node.data.name == "subtestfile" {
                assert_eq!(
                    node.data.ftype,
                    crate::docker_file_tree::DDiveFileType::File
                );
                assert_eq!(node.data.fop, crate::docker_file_tree::FileOp::Add);
                assert_eq!(
                    node.data.disk_rel_path.to_str().unwrap(),
                    "/subtest/subtestfile"
                );
                assert_eq!(
                    node.data.vis_rel_path.to_str().unwrap(),
                    "/subtest/subtestfile"
                );
            }
            if node.data.name == "whatever" {
                assert_eq!(
                    node.data.ftype,
                    crate::docker_file_tree::DDiveFileType::File
                );
                assert_eq!(node.data.fop, crate::docker_file_tree::FileOp::Remove);
                assert_eq!(
                    node.data.disk_rel_path.to_str().unwrap(),
                    "/subtest2/.wh.whatever"
                );
                assert_eq!(
                    node.data.vis_rel_path.to_str().unwrap(),
                    "/subtest2/whatever"
                );
            }
        }
    }

    #[test]
    fn tree_ser_deser() {
        let tree = construct_tree();
        let serialised = serde_json::to_string(&tree).unwrap();
        let deserialised: FileTree = serde_json::from_str(&serialised).unwrap();
        assert_eq!(tree.parent_node, deserialised.parent_node);
    }

    #[test]
    fn tree_filter() {
        let tree = construct_tree();
        let (filtered_tree, err) = tree.filter_tree_full_path("subtest");
        assert_matches!(err, None);
        // First three nodes should be subtest, subtest2 and subtest3
        let root = filtered_tree.root();
        let names = root.borrow().get_children_names();

        for name in names.iter() {
            println!("Name: {}", name);
        }

        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "subtest");
        assert_eq!(names[1], "subtest2");
        assert_eq!(names[2], "subtest3");

        // Check that after filter children are present
        let subtest_node = tree.get_node_by_name("subtest");
        assert_matches!(subtest_node, Some(_));
        let subtest_node = subtest_node.unwrap();
        let subtest_children = subtest_node.borrow().get_children_names();
        assert_eq!(subtest_children.len(), 2);
        assert_eq!(subtest_children[0], "subsubtest");
        assert_eq!(subtest_children[1], "subtestfile");

        for name in subtest_children.iter() {
            println!("Name: {}", name);
        }
    }

    #[test]
    fn tree_filter_long() {
        let tree = construct_tree();
        let (filtered_tree, err) = tree.filter_tree_full_path("subtest/subsub");
        assert_matches!(err, None);
        // First three nodes should be subtest, subtest2 and subtest3
        let root = filtered_tree.root();
        let names = root.borrow().get_children_names();

        for name in names.iter() {
            println!("Name: {}", name);
        }

        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "subtest");

        // Check that after filter children are present
        let subtest_node = tree.get_node_by_name("subtest");
        assert_matches!(subtest_node, Some(_));
        let subtest_node = subtest_node.unwrap();
        let subtest_children = subtest_node.borrow().get_children_names();
        assert_eq!(subtest_children.len(), 2);
        assert_eq!(subtest_children[0], "subsubtest");
        assert_eq!(subtest_children[1], "subtestfile");

        for name in subtest_children.iter() {
            println!("Name: {}", name);
        }
    }

    #[test]
    fn real_life_tree() {
        let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_file = project_dir.join("test-assets/real-life-layers/sha256:270a1170e7e398434ff1b31e17e233f7d7b71aa99a40473615860068e86720af.json");
        let layer_cache_file = File::open(&test_file).unwrap();
        let layer_tree: FileTree = serde_json::from_reader(layer_cache_file).unwrap();
        let (filtered_tree, error) = layer_tree.filter_tree_full_path("/etc/");
        assert_matches!(error, None);
        let root = filtered_tree.root();
        let children_names = root.borrow().get_children_names();
        assert_eq!(children_names.len(), 1);
        assert_eq!(children_names[0], "etc");
        let etc_node = root.borrow().get_child(0).unwrap();
        assert!(etc_node.borrow().get_n_children() > 0);
        let etc_children = etc_node.borrow().get_children_names();
        let apt_cache_name = etc_children.iter().position(|name| name == "apt").unwrap();
        let apt_node = etc_node.borrow().get_child(apt_cache_name).unwrap();
        let trusted = apt_node
            .borrow()
            .get_children_names()
            .iter()
            .position(|name| name == "trusted.gpg.d")
            .unwrap();
        let trusted_node = apt_node.borrow().get_child(trusted).unwrap();
        assert_eq!(trusted_node.borrow().get_n_children(), 2);
    }
}
