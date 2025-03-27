mod docker_file_tree;
mod docker_image_utils;

use docker_image_utils::{ImageRepr};
use bollard::Docker;
use std::error::Error;
use clap::{command,Arg};






#[tokio::main]
async fn main() -> Result<(), Box<dyn Error> >{
    // let args = Args::parse();
    let matches = command!() // requires `cargo` feature
    .arg(Arg::new("Name"))
    .get_matches();

    let img_name : String = matches.get_one::<String>("Name").expect("Can't parse to string").clone();

    let docker = Docker::connect_with_socket_defaults().expect("Can't connect to docker");

    let img = ImageRepr::new(img_name, &docker).await;

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
