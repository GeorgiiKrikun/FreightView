use bollard::{secret::ImageSummary, Docker};
use bollard::image::ListImagesOptions;
use std::default::Default;
use std::path::PathBuf;
use clap::{command,Arg};
// use std::default::Default;
use std::fs::File;
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


    // Unpack layers
    for layer  in layers {
        let layer_spec = &layer[7..];
        let layer_path = format!("image/blobs/sha256/{}", layer_spec);
        
        let layer_tar = File::open(layer_path).expect("Can't open layer");
        let out_layer_path = format!("image/blobs/sha256/{}.dir/", layer_spec);

        println!("Unpacking layer: {} to {}", layer, out_layer_path);
        let mut layer_archive = Archive::new(layer_tar);
        layer_archive.unpack(out_layer_path).expect("Can't unpack layer");       
    }

    // Walk through the image

    
    for entry in WalkDir::new("image") {
        let entry = entry.expect("Can't get entry");
        println!("{}", entry.path().display());
    }


    

    // 


    // // Print the image layers and their contents


}
