use bollard::{secret::ImageSummary, Docker};
use bollard::image::ListImagesOptions;
use std::default::Default;
use clap::{command,Arg};

async fn get_image_summary(docker: &Docker, img_name: &str) -> ImageSummary {
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

    image.expect("Image was not found")
}

#[tokio::main]
async fn main() {
    // let args = Args::parse();
    let matches = command!() // requires `cargo` feature
    .arg(Arg::new("Image name"))
    .get_matches();

    let img_name : &String = matches.get_one::<String>("Image name").expect("Can't parse to string");
    println!("Image name: {}", img_name);

    img_sum : ImageSummary = get_image_summary(docker, img_name)

    

    

    // println!("Found image summary {:?}", image);

    // let image_details = docker.inspect_image(img_name).await.unwrap();

    // // Print the image layers and their contents
    // if let Some(root_fs) = image_details.root_fs {
    //     if let Some(layers) = root_fs.layers {
    //         println!("Image layers:");
    //         for layer in layers {
    //             println!("Layer: {}", layer);
    //         }
    //     }
    // } else {
    //     println!("No layers found for the image.");
    // }

}
