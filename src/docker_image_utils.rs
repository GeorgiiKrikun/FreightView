use bollard::{secret::ImageSummary, Docker, image::ListImagesOptions};
use futures_util::StreamExt;
use futures_core::task::Poll;
use home::home_dir;
use tempfile::TempDir;
use std::mem::swap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tar::Archive;
use crate::docker_file_tree::{TreeNode, DDiveFileType, FileOp, parse_directory_into_tree};

pub struct ImageLayer {
    pub name: String,
    pub tree: TreeNode,
}

impl ImageLayer {
    pub fn new(name: String, tree: TreeNode) -> ImageLayer {
        ImageLayer {
            name: name,
            tree: tree,
        }
    }

    fn get_cache_dir() -> PathBuf {
        let home = home_dir().expect("Can't get home directory");
        let ddive_cache_path = home.join(".ddive");
        if !ddive_cache_path.exists() {
            std::fs::create_dir(&ddive_cache_path).expect("Can't create ddive cache directory");
        }
        ddive_cache_path
    }

    fn get_layer_path_wstr(layer : &str) -> PathBuf {
        ImageLayer::get_cache_dir().join(layer).with_extension("json")
    }

    fn get_layer_path(&self) -> PathBuf {
        ImageLayer::get_layer_path_wstr(&self.name)
    }

    pub fn save(&self) {
        let layer_cache_path = self.get_layer_path();
        let mut layer_cache_file = File::create(&layer_cache_path).expect("Can't create layer cache file");
        let layer_json = serde_json::to_string(&self.tree).expect("Can't serialize layer tree");
        layer_cache_file.write_all(layer_json.as_bytes()).expect("Can't write to layer cache file");        
    }

    pub fn load(layer : &str) -> Result<ImageLayer, Box<dyn std::error::Error> > {
        let layer_cache_path = ImageLayer::get_layer_path_wstr(layer);
        let layer_cache_file = File::open(&layer_cache_path)?;
        let layer_tree : TreeNode = serde_json::from_reader(layer_cache_file)?;
        Ok(ImageLayer::new(layer.to_string(), layer_tree))
    }

    pub fn check_cache(layer : &str ) -> bool {
        ImageLayer::get_layer_path_wstr(layer).exists()
    }

    pub fn filter_cached_layers(layers : &mut Vec<String>) -> Vec<ImageLayer> {
        let mut remaining_layers: Vec<String> = Vec::new();
        let mut cached_layers: Vec<String> = Vec::new();
        let mut out_layers : Vec<ImageLayer> = Vec::new();
        for layer  in &mut *layers {
            if ImageLayer::check_cache(layer) {
                println!("Layer {} is cached", layer);
                cached_layers.push(layer.clone());
            } else {
                println!("Layer {} is not cached", layer);
                remaining_layers.push(layer.clone());
            }
        }

        for layer in cached_layers {
            let layer = ImageLayer::load(&layer).expect("Can't load layer");
            out_layers.push(layer);
        }

        swap(layers, &mut remaining_layers);

        out_layers

    }
}

pub struct ImageRepr {
    name : String,
    temp_dir : TempDir,
    pub layers : Vec<ImageLayer>,
}

impl ImageRepr {
    pub async fn new(name: String, docker: &Docker) -> ImageRepr {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let img_tar_file_path = PathBuf::from(temp_dir.path()).join("image.tar");
        let img_folder = PathBuf::from(temp_dir.path()).join("image");
        let layers : Vec<String> = download_image_file(&docker, &name, &img_tar_file_path).await;

        // Untar image
        let file: File = File::open(&img_tar_file_path).expect("Can't open file");
        let mut archive = Archive::new(file);
        archive.unpack(&img_folder).expect("Can't unpack image");

        let layer_folder = img_folder.join("blobs").join("sha256");

        // split layers into cached and non-cached
        let mut non_cached_layers = layers.clone();

        // Get cached layers
        let _ = ImageLayer::filter_cached_layers(&mut non_cached_layers);

        // get non-cached layers
        let layer_trees: Vec<(String, TreeNode)> = unpack_image_layers(&layer_folder, &non_cached_layers);

        // Construct cache from non-cached layers
        for (layer_name, layer_tree) in layer_trees {
            let layer = ImageLayer::new(layer_name, layer_tree);
            layer.save();
        }

        // Finally, load all layers from cache
        let mut all_layers : Vec<ImageLayer> = Vec::new();
        for layer in layers {
            let layer = ImageLayer::load(&layer).expect("Can't load layer");
            all_layers.push(layer);
        }
        
        ImageRepr {
            name: name,
            temp_dir: temp_dir,
            layers: all_layers,
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
        let layer_dir_name = String::from(layer) + ".dir";
        let layer_dir = layer_folder.join(&layer_dir_name);
        
        let layer_tar = File::open(&layer_tar_path).expect("Can't open layer");

        println!("Unpacking layer: {} to {}", layer, layer_dir.display());
        let mut layer_archive = Archive::new(layer_tar);
        layer_archive.unpack(&layer_dir).expect("Can't unpack layer");       

        // stupid crap but idk how to do it yet

        let main_dir = layer_dir.clone();
        let mut layer_tree = TreeNode::new(&DDiveFileType::Directory, &FileOp::Add, &main_dir);
        parse_directory_into_tree(&main_dir, layer_dir, &mut layer_tree);
        let mut layer_tree = layer_tree.prettyfy();
        // Change root of the tree to the first child
        layer_trees.push((layer.to_string(), layer_tree));
    }

    layer_trees
}