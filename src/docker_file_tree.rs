use serde::{Deserialize, Serialize};
use std::fs::FileType;
use std::{collections::VecDeque, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum DDiveFileType {
    Directory,
    File,
    Symlink,
    Badfile,
}

impl DDiveFileType {
    pub fn from_ftype(ftype: FileType) -> DDiveFileType {
        if ftype.is_symlink() {
            // Symlink needs to go first, since the symlink can lead somewhere outside the docker
            DDiveFileType::Symlink
        } else if ftype.is_file() {
            DDiveFileType::File
        } else if ftype.is_dir() {
            DDiveFileType::Directory
        } else {
            DDiveFileType::Badfile
        }
    }
}

// File operations type
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum FileOp {
    Add,
    Remove,
}

impl std::fmt::Display for FileOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOp::Add => write!(f, "Add"),
            FileOp::Remove => write!(f, "Remove"),
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct TreeNode {
    path: PathBuf,
    ftype: DDiveFileType,
    fop: FileOp,
    kids: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(ftype: &DDiveFileType, fop: &FileOp, path: &PathBuf) -> TreeNode {
        TreeNode {
            path: path.clone(),
            ftype: ftype.clone(),
            fop: fop.clone(),
            kids: Vec::new(),
        }
    }

    pub fn kids(&self) -> Vec<&TreeNode> {
        let mut vec: Vec<&TreeNode> = Vec::new();
        for kid in &self.kids {
            vec.push(kid);
        }
        vec
    }

    pub fn prettyfy(mut self) -> TreeNode {
        let out_node: TreeNode;
        if self.kids.len() == 1 {
            let mut kid = self.kids.remove(0);
            if kid.path == PathBuf::from("") {
                kid.path = PathBuf::from("/");
            }
            out_node = kid;
        } else {
            println!(
                "Prettyfying node with {} children is not feasible",
                self.kids.len()
            );
            out_node = self;
        }
        out_node
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    #[allow(dead_code)]
    pub fn ftype(&self) -> &DDiveFileType {
        &self.ftype
    }

    #[allow(dead_code)]
    pub fn fop(&self) -> &FileOp {
        &self.fop
    }

    #[allow(dead_code)]
    pub fn is_leaf(&self) -> bool {
        self.kids.is_empty()
    }

    pub fn breadth_first(&self) -> Vec<&TreeNode> {
        let mut nodes: Vec<&TreeNode> = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(self);
        while !queue.is_empty() {
            let node = queue.pop_front().unwrap();
            nodes.push(node);
            for kid in node.kids() {
                queue.push_back(kid);
            }
        }
        nodes
    }

    pub fn add_child(&mut self, child: TreeNode) -> &mut TreeNode {
        self.kids.push(child);
        self.kids.last_mut().unwrap()
    }

    #[allow(dead_code)]
    pub fn print_tree(&self, depth: usize, output: &mut String) {
        let mut indent = String::new();
        for _ in 0..depth {
            indent.push_str("  ");
        }
        let ftype_str = match &self.ftype {
            DDiveFileType::Directory => "Directory",
            DDiveFileType::File => "File",
            DDiveFileType::Symlink => "Symlink",
            DDiveFileType::Badfile => "Badfile",
        };
        let fop_str = match &self.fop {
            FileOp::Add => "Add",
            FileOp::Remove => "Remove",
        };
        let path_str = self.path.to_str().unwrap();
        output.push_str(&format!(
            "{}{}<{}>: {}\n",
            indent, ftype_str, path_str, fop_str
        ));
        // println!("{}{}<{}>: {}", indent, ftype_str, path_str, fop_str);
        for kid in &self.kids {
            kid.print_tree(depth + 1, output);
        }
    }

    pub fn filter_tree_full_path(&self, filter: &str) -> Option<TreeNode> {
        let filter: Vec<&str> = filter.split("/").collect();
        // clean up empty strings
        let filter: Vec<&str> = filter.iter().filter(|&x| x != &"").map(|x| *x).collect();
        if filter.len() == 0 {
            return None;
        }
        let mut out: TreeNode = self.clone();
        let mut current: &mut TreeNode = &mut out;
        // last search string should be taken care separately as it should not filter when the path is not
        // yet fully typed
        for d in 0..filter.len() - 1 {
            let subfilter: &str = filter[d];
            let mut next_ind: Option<usize> = None;
            let current_nkids = current.kids.len();

            for i in 0..current_nkids {
                if current.kids()[i].path().file_name().unwrap() == subfilter {
                    next_ind = Some(i);
                    break;
                }
            }

            match next_ind {
                Some(n) => {
                    // extract filtered node
                    let extracted = current.kids.swap_remove(n);
                    // remove all other nodes
                    current.kids.clear();
                    // add filtered node
                    current.kids.push(extracted);
                    // move to next node
                    current = current.kids.last_mut().unwrap();
                }
                None => {
                    return None;
                }
            }
        }

        // Parse the last string after / to filter the last node
        let subfilter: &str = filter[filter.len() - 1];
        let mut inds: Vec<usize> = Vec::new();
        let current_nkids: usize = current.kids.len();
        for i in 0..current_nkids {
            if current.kids()[i]
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(subfilter)
            {
                inds.push(i);
            }
        }

        if inds.len() == 0 {
            return None;
        }

        let mut new_kids: Vec<TreeNode> = Vec::new();
        for i in inds {
            new_kids.push(current.kids[i].clone());
        }

        current.kids = new_kids;

        Some(out)
    }
}

// Parse directory into tree
pub fn parse_directory_into_tree(main_path: &PathBuf, path: PathBuf, parent: &mut TreeNode) {
    let rel_path = PathBuf::from(path.strip_prefix(main_path).unwrap());
    let metadata = std::fs::metadata(&path);
    if metadata.is_err() {
        println!("Error reading metadata for path: {}", &path.display());
        return;
    }
    let metadata = metadata.unwrap();

    let ftype = DDiveFileType::from_ftype(metadata.file_type());
    let node_path = rel_path.clone();

    let mut file_name = match node_path.file_name() {
        Some(name) => name.to_str().unwrap(),
        None => "",
    };

    let dir_name = node_path.parent();
    let mut dir_name: String = match dir_name {
        Some(name) => String::from(name.to_str().unwrap()),
        None => String::from(""),
    };

    let op = if file_name.starts_with(".wh.") {
        file_name = file_name.strip_prefix(".wh.").unwrap();
        FileOp::Remove
    } else {
        FileOp::Add
    };

    if !dir_name.starts_with("/") {
        dir_name = format!("/{}", dir_name);
    }

    if !dir_name.ends_with("/") {
        dir_name = format!("{}/", dir_name);
    }

    let final_path = PathBuf::from(format!("{}{}", dir_name, file_name));

    let node = TreeNode::new(&ftype, &op, &final_path);
    let node = parent.add_child(node);

    match &ftype {
        &DDiveFileType::Directory => {
            let entries = std::fs::read_dir(&path);
            match entries {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(pentry) => {
                                let entry_path = pentry.path();
                                parse_directory_into_tree(main_path, entry_path, node);
                            }
                            Err(e) => {
                                println!("Erroroneous dir entry: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Error reading directory {}: {}", &path.display(), e);
                    return;
                }
            }
        }
        &DDiveFileType::File => {
            // Do nothing
        }
        &DDiveFileType::Symlink => {
            // Do nothing
        }
        &DDiveFileType::Badfile => {
            // Do nothing
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::parse_directory_into_tree;

    fn construct_tree() -> super::TreeNode {
        let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = project_dir.join("test-files");
        let mut parent = super::TreeNode::new(
            &super::DDiveFileType::Directory,
            &super::FileOp::Add,
            &PathBuf::from(""),
        );
        parse_directory_into_tree(&test_dir, test_dir.clone(), &mut parent);
        let parent = parent.prettyfy();
        return parent;
    }

    #[test]
    fn test_print_tree() {
        let tree = construct_tree();
        let mut out = String::new();
        tree.print_tree(0, &mut out);
        println!("{}", out);
    }

    #[test]
    fn filter_tree_full_path() {
        let tree = construct_tree();
        let filter = "/subtest/subsubtest";
        let filtered_tree = tree.filter_tree_full_path(filter);
        let mut out = String::new();
        filtered_tree.unwrap().print_tree(0, &mut out);
        let out = out.split("\n").collect::<Vec<&str>>();
        assert_eq!(out[0].trim(), "Directory</>: Add");
        assert_eq!(out[1].trim(), "Directory</subtest>: Add");
        assert_eq!(out[2].trim(), "Directory</subtest/subsubtest>: Add");
        assert_eq!(
            out[3].trim(),
            "File</subtest/subsubtest/subsubtestfile>: Add"
        );
    }

    #[test]
    fn filter_tree_full_path_2() {
        let tree = construct_tree();
        let filter = "/subtest";
        let filtered_tree = tree.filter_tree_full_path(filter);
        let mut out = String::new();
        filtered_tree.unwrap().print_tree(0, &mut out);
        let out = out.split("\n").collect::<Vec<&str>>();
        assert_eq!(out[0].trim(), "Directory</>: Add");
        assert_eq!(out[1].trim(), "Directory</subtest>: Add");
        assert_eq!(out[2].trim(), "File</subtest/subtestfile>: Add");
        assert_eq!(out[3].trim(), "Directory</subtest/subsubtest>: Add");
        assert_eq!(
            out[4].trim(),
            "File</subtest/subsubtest/subsubtestfile>: Add"
        );
        assert_eq!(out[5].trim(), "Directory</subtest/subsubtest2>: Add");
    }

    #[test]
    fn filter_tree_full_path_none() {
        let tree = construct_tree();
        let filter = "/subtest/subsasdubtest2";
        let filtered_tree = tree.filter_tree_full_path(filter);
        match filtered_tree {
            Some(_) => {
                assert!(false);
            }
            None => {
                assert!(true);
            }
        }
    }

    #[test]
    fn filter_tree_not_full() {
        let tree = construct_tree();
        let filter = "/subtest/subsubte";
        let filtered_tree = tree.filter_tree_full_path(filter);
        let mut out = String::new();
        filtered_tree.unwrap().print_tree(0, &mut out);
        let out = out.split("\n").collect::<Vec<&str>>();
        for line in &out {
            println!("{}", line);
        }

        assert_eq!(out[0].trim(), "Directory</>: Add");
        assert_eq!(out[1].trim(), "Directory</subtest>: Add");
        assert_eq!(out[2].trim(), "Directory</subtest/subsubtest>: Add");
        assert_eq!(
            out[3].trim(),
            "File</subtest/subsubtest/subsubtestfile>: Add"
        );
        assert_eq!(out[4].trim(), "Directory</subtest/subsubtest2>: Add");
    }

    #[test]
    fn filter_partial_start() {
        let tree = construct_tree();
        let filter = "/subte";
        let filtered_tree = tree.filter_tree_full_path(filter);
        let mut out = String::new();
        filtered_tree.unwrap().print_tree(0, &mut out);
        let out = out.split("\n").collect::<Vec<&str>>();
        for line in &out {
            println!("{}", line);
        }

        let outputs = [
            "Directory</>: Add",
            "Directory</subtest>: Add",
            "File</subtest/subtestfile>: Add",
            "Directory</subtest/subsubtest>: Add",
            "File</subtest/subsubtest/subsubtestfile>: Add",
            "Directory</subtest/subsubtest2>: Add",
            "Directory</subtest2>: Add",
            "File</subtest2/whatever>: Remove",
            "File</subtest2/subfile2>: Add",
            "Directory</subtest2/subsubtest2>: Add",
            "File</subtest2/subsubtest2/subsubfile2>: Add",
        ];

        for i in 0..outputs.len() {
            assert_eq!(out[i].trim(), outputs[i]);
        }
    }
}
