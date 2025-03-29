use std::{
    collections::VecDeque, 
    path::PathBuf
};
use std::fs::FileType;
use serde::{
    Deserialize, 
    Serialize
};

#[derive(Clone,Serialize,Deserialize, Eq, PartialEq, Hash)]
pub enum DDiveFileType {
    Directory,
    File,
    Symlink,
    Badfile
}

impl DDiveFileType {
    pub fn from_ftype(ftype: FileType) -> DDiveFileType {
        if ftype.is_dir() {
            DDiveFileType::Directory
        } else if ftype.is_file() {
            DDiveFileType::File
        } else if ftype.is_symlink() {
            DDiveFileType::Symlink
        } else {
           DDiveFileType::Badfile
        }
    }
}

// File operations type
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum FileOp {
    Add,
    Remove
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct TreeNode {
    path: PathBuf,
    ftype: DDiveFileType,
    fop: FileOp,
    kids: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(ftype : &DDiveFileType, fop: &FileOp, path: &PathBuf) -> TreeNode {
        TreeNode {
            path: path.clone(),
            ftype: ftype.clone(),
            fop: fop.clone(),
            kids: Vec::new(),
        }
    }

    pub fn kids(&self) -> Vec<&TreeNode> {
        let mut vec : Vec<&TreeNode> = Vec::new();
        for kid in &self.kids {
            vec.push(kid);
        }
        vec
    }

    pub fn prettyfy(mut self) -> TreeNode {
        if self.kids.len() == 1 {
            let mut kid = self.kids.remove(0);
            if kid.path == PathBuf::from("") {
                kid.path = PathBuf::from("/");
            }
            return kid;
        } else {
            println!("Prettyfying node with {} children is not feasible", self.kids.len());
            return self;
        }

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
        let mut nodes : Vec<&TreeNode> = Vec::new();
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

    pub fn print_tree(&self, depth: usize) {
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
        println!("{}{}<{}>: {}", indent, ftype_str, path_str, fop_str);
        for kid in &self.kids {
            kid.print_tree(depth + 1);
        }
    }
}

const PATH_BLACKLIST : [&str; 2] = ["var/run", "run"];
fn is_blacklisted(path: &str) -> bool {
    PATH_BLACKLIST.contains(&path)
}

// Parse directory into tree
pub fn parse_directory_into_tree(main_path: &PathBuf, path: PathBuf, parent : &mut TreeNode) {
    let rel_path = PathBuf::from(path.strip_prefix(main_path).unwrap());
    if is_blacklisted(rel_path.to_str().unwrap()) {
        println!("Entered blacklisted path: {}", rel_path.display());
        return;
    }
    let metadata = std::fs::metadata(&path);
    if metadata.is_err() {
        println!("Error reading metadata for path: {}", &path.display());
        return;
    }
    let metadata = metadata.unwrap();

    let ftype = DDiveFileType::from_ftype(metadata.file_type());
    let node = TreeNode::new(&ftype, &FileOp::Add, &rel_path);
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