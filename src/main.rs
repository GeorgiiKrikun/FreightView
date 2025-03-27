mod docker_file_tree;
mod docker_image_utils;

use docker_file_tree::{DDiveFileType, FileOp, TreeNode, parse_directory_into_tree};
use docker_image_utils::{download_image_file, unpack_image_layers,  ImageRepr};
use bollard::Docker;
use std::error::Error;
use std::path::PathBuf;
use clap::{command,Arg};
use std::fs::File;
use std::io::Write;
use tar::Archive;

use tempfile::TempDir;
use serde_json;
use home::home_dir;





#[tokio::main]
async fn main() -> Result<(), Box<dyn Error> >{
    // let args = Args::parse();
    let matches = command!() // requires `cargo` feature
    .arg(Arg::new("Name"))
    .get_matches();

    let img_name : String = matches.get_one::<String>("Name").expect("Can't parse to string").clone();

    let docker = Docker::connect_with_socket_defaults().expect("Can't connect to docker");

    let img = ImageRepr::new(img_name, &docker).await;

    let home = home_dir().expect("Can't get home directory");
    let ddive_cache_path = home.join(".ddive");
    // ensure ddive cache path exists
    if !ddive_cache_path.exists() {
        std::fs::create_dir(&ddive_cache_path).expect("Can't create ddive cache directory");
    }

    for (layer_spec, layer_tree) in &img.layers {
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
