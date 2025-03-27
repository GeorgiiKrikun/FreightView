use bollard::{secret::ImageSummary, Docker, image::ListImagesOptions};
use futures_util::StreamExt;
use futures_core::task::Poll;
use tempfile::TempDir;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tar::Archive;
use crate::docker_file_tree::{TreeNode, DDiveFileType, FileOp, parse_directory_into_tree};

pub struct ImageRepr {
    name : String,
    temp_dir : TempDir,
    pub layers : Vec<(String,TreeNode)>,
}

impl ImageRepr {
    pub async fn new(name: String, docker: &Docker) -> ImageRepr {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let img_tar_file_path = PathBuf::from(temp_dir.path()).join("image.tar");
        let img_folder = PathBuf::from(temp_dir.path()).join("image");
        let layers = download_image_file(&docker, &name, &img_tar_file_path).await;

        // Untar image
        let file: File = File::open(&img_tar_file_path).expect("Can't open file");
        let mut archive = Archive::new(file);
        archive.unpack(&img_folder).expect("Can't unpack image");

        let layer_folder = img_folder.join("blobs").join("sha256");

        let layer_trees: Vec<(String, TreeNode)> = unpack_image_layers(&layer_folder, &layers);


        ImageRepr {
            name: name,
            temp_dir: temp_dir,
            layers: layer_trees,
        }
    }


}

pub async fn get_image_summary(docker: &Docker, img_name: &String) -> Option<ImageSummary> {
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

pub async fn download_image_file(docker: &Docker, img_name: &String, img_tar_file_path : &PathBuf) -> Vec<String> {
    let image_details = docker.inspect_image(img_name).await.unwrap();
    let rfs = image_details.root_fs.expect("No fs");
    let layers = rfs.layers.expect("No layers in the image");
    let mut stream = docker.export_image(img_name);
    let mut context = std::task::Context::from_waker(futures_util::task::noop_waker_ref());
    let mut file = File::create(&img_tar_file_path).expect("Can't create file");


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

    layers
   
}

pub fn unpack_image_layers(layer_folder: &PathBuf, layers: &Vec<String> ) -> Vec<(String,TreeNode)> {
    let mut layer_trees: Vec<(String,TreeNode)> = Vec::new();

    for layer  in layers {
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

    layer_trees
}