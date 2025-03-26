use bollard::{secret::ImageSummary, Docker};
use bollard::image::ListImagesOptions;
use std::default::Default;
use clap::{command,Arg};
// use std::default::Default;
use std::fs::File;
use std::io::Write;
use flate2::read::GzDecoder;
use tar::Archive;
use futures_util::StreamExt;
use futures_core::task::Poll;

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
    let mut file = File::create("image.tar").expect("Can't create file");

    let mut context = std::task::Context::from_waker(futures_util::task::noop_waker_ref());

    while let poll_res = stream.poll_next_unpin(&mut context) {
        match poll_res {
            Poll::Ready(option) => {
                match option {
                    Some(errchnk) => {
                        let chnk = errchnk.expect("Can't get chunk");
                        file.write_all(&chnk).expect("Can't write to file");
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
    let tar = File::open("image.tar").expect("Can't open file");
    let mut archive = Archive::new(tar);
    archive.unpack("image").expect("Can't unpack image");

    for layer  in layers {
        let layer_spec = &layer[7..];
        let layer_path = format!("image/blobs/sha256/{}", layer_spec);
        
        let layer_tar = File::open(layer_path).expect("Can't open layer");
        let out_layer_path = format!("image/blobs/sha256/{}.dir/", layer_spec);

        println!("Unpacking layer: {} to {}", layer, out_layer_path);
        let mut layer_archive = Archive::new(layer_tar);
        layer_archive.unpack(out_layer_path).expect("Can't unpack layer");       
    }



    

    // 


    // // Print the image layers and their contents


}
