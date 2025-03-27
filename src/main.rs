mod docker_file_tree;
mod docker_image_utils;

use docker_file_tree::{DDiveFileType, FileOp, TreeNode, parse_directory_into_tree};

use bollard::Docker;
use std::error::Error;
use std::path::PathBuf;
use clap::{command,Arg};
use std::fs::File;
use std::io::Write;
use tar::Archive;
use futures_util::StreamExt;
use futures_core::task::Poll;
use tempfile::TempDir;
use serde_json;
use home::home_dir;





#[tokio::main]
async fn main() -> Result<(), Box<dyn Error> >{
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

    let home = home_dir().expect("Can't get home directory");
    let ddive_cache_path = home.join(".ddive");
    // ensure ddive cache path exists
    if !ddive_cache_path.exists() {
        std::fs::create_dir(&ddive_cache_path).expect("Can't create ddive cache directory");
    }

    for (layer_spec, layer_tree) in layer_trees {
        println!("Layer: {}", layer_spec);
        let layer_json = serde_json::to_string(&layer_tree).expect("Can't serialize layer tree");
        let layer_cache_path = ddive_cache_path.join(&layer_spec).with_extension("json");
        let mut layer_cache_file = File::create(&layer_cache_path).expect("Can't create layer cache file");
        layer_cache_file.write_all(layer_json.as_bytes()).expect("Can't write to layer cache file");
    }

    Ok(())

    // Walk through the image
    
    
    // for layer in &layers {
    //     let layer_spec = &layer[7..];
    //     let layer_dir_name = String::from(layer_spec) + ".dir";
    //     let layer_dir = layer_folder.join(&layer_dir_name);
        
        
    // }

    


    

    // 


    // // Print the image layers and their contents


}
