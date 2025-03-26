use bollard::{secret::ImageSummary, Docker};
use bollard::image::ListImagesOptions;
use clap::builder::Str;
use std::default::Default;
use std::path::{self, Path, PathBuf};
use clap::{command,Arg};
// use std::default::Default;
use std::fs::{File, FileType};
use std::io::Write;
use flate2::read::GzDecoder;
use tar::Archive;
use futures_util::StreamExt;
use futures_core::task::Poll;
use walkdir::WalkDir;
use tempfile::TempDir;

async fn get_image_summary(docker: &Docker, img_name: &String) -> Option<ImageSummary> {
    let images = &docker.list_images(Some(ListImagesOptions::<String> {
        all: true,
        ..Default::default()
    })).await.unwrap();
    
    let mut image : Option<ImageSummary> = None;
    for img in images {
        if img.repo_tags.contains(img_name) {
            image = Some(img.clone());
            break;
        }
    }

    image
}

#[derive(Clone)]
enum DDiveFileType {
    Directory,
    File,
    Symlink
}

impl DDiveFileType {
    fn from_ftype(ftype: FileType) -> DDiveFileType {
        if ftype.is_dir() {
            DDiveFileType::Directory
        } else if ftype.is_file() {
            DDiveFileType::File
        } else if ftype.is_symlink() {
            DDiveFileType::Symlink
        } else {
            panic!("Unknown file type");
        }
    }
}

// File operations type
#[derive(Clone)]
enum FileOp {
    Add,
    Remove
}

struct TreeNode {
    kids: Vec<TreeNode>,
    ftype: DDiveFileType,
    fop: FileOp,
    path: PathBuf,
}

impl TreeNode {
    fn new(ftype : &DDiveFileType, fop: &FileOp, path: &PathBuf) -> TreeNode {
        TreeNode {
            kids: Vec::new(),
            ftype: ftype.clone(),
            fop: fop.clone(),
            path: path.clone(),
        }
    }

    fn add_child(&mut self, child: TreeNode) -> &mut TreeNode {
        self.kids.push(child);
        self.kids.last_mut().unwrap()
    }

    fn print_tree(&self, depth: usize) {
        let mut indent = String::new();
        for _ in 0..depth {
            indent.push_str("  ");
        }
        let ftype_str = match &self.ftype {
            DDiveFileType::Directory => "Directory",
            DDiveFileType::File => "File",
            DDiveFileType::Symlink => "Symlink",
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
fn parse_directory_into_tree(main_path: &PathBuf, path: PathBuf, parent : &mut TreeNode) {
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
    let mut node = TreeNode::new(&ftype, &FileOp::Add, &rel_path);
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
    }
}


#[tokio::main]
async fn main() {
    // let args = Args::parse();
    let matches = command!() // requires `cargo` feature
    .arg(Arg::new("Name"))
    .get_matches();

    let img_name : &String = matches.get_one::<String>("Name").expect("Can't parse to string");
    println!("Image name: {}", img_name);

    let docker = Docker::connect_with_socket_defaults().expect("Can't connect to docker");

    let image_details = docker.inspect_image(img_name).await.unwrap();
    let rfs = image_details.root_fs.expect("No fs");
    let layers = rfs.layers.expect("No layers in the image");

    let mut stream = docker.export_image(img_name);
    let mut context = std::task::Context::from_waker(futures_util::task::noop_waker_ref());

    let binding = TempDir::new().expect("Failed to create temporary directory");
    let temp_dir = binding.path();
    let img_tar_file_path = PathBuf::from(temp_dir).join("image.tar");
    let img_folder = PathBuf::from(temp_dir).join("image");
    println!("Image tar file: {}", img_tar_file_path.display());
    let mut img_tar_file = File::create(&img_tar_file_path).expect("Can't create file");


    // println!("Temporary directory created at: {}", temp_dir_path.display());

    while let poll_res = stream.poll_next_unpin(&mut context) {
        match poll_res {
            Poll::Ready(option) => {
                match option {
                    Some(errchnk) => {
                        let chnk = errchnk.expect("Can't get chunk");
                        img_tar_file.write_all(&chnk).expect("Can't write to file");
                    }
                    None => {
                        break;
                    }
                }
            },
            Poll::Pending => {
                continue;
            }
        }
    }

    // Untar image
    let file = File::open(&img_tar_file_path).expect("Can't open file");
    let mut archive = Archive::new(file);
    archive.unpack(&img_folder).expect("Can't unpack image");


    let layer_folder = img_folder.join("blobs").join("sha256");
    println!("Layer folder: {}", layer_folder.display());

    let mut layer_trees: Vec<(String,TreeNode)> = Vec::new();

    // Unpack layers
    for layer  in &layers {
        let layer_spec = &layer[7..];
        let layer_tar_path = layer_folder.join(layer_spec);
        let layer_dir_name = String::from(layer_spec) + ".dir";
        let layer_dir = layer_folder.join(&layer_dir_name);
        
        let layer_tar = File::open(&layer_tar_path).expect("Can't open layer");

        println!("Unpacking layer: {} to {}", layer, layer_dir.display());
        let mut layer_archive = Archive::new(layer_tar);
        layer_archive.unpack(&layer_dir).expect("Can't unpack layer");       

        // stupid crap but idk how to do it yet

        let main_dir = layer_dir.clone();
        let mut layer_tree = TreeNode::new(&DDiveFileType::Directory, &FileOp::Add, &main_dir);
        parse_directory_into_tree(&main_dir, layer_dir, &mut layer_tree);
        layer_trees.push((layer_spec.to_string(), layer_tree));
    }

    for (layer_spec, layer_tree) in layer_trees {
        println!("Layer: {}", layer_spec);
        layer_tree.print_tree(0);
    }

    // Walk through the image
    
    
    // for layer in &layers {
    //     let layer_spec = &layer[7..];
    //     let layer_dir_name = String::from(layer_spec) + ".dir";
    //     let layer_dir = layer_folder.join(&layer_dir_name);
        
        
    // }

    


    

    // 


    // // Print the image layers and their contents


}
