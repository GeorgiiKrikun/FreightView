use bollard::{
    secret::ImageSummary, 
    Docker, 
    image::ListImagesOptions
};
use futures_util::StreamExt;
use futures_core::task::Poll;
use home::home_dir;
use tempfile::TempDir;
use std::{io::Read, mem::swap};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tar::Archive;
use crate::docker_file_tree::{
    TreeNode, 
    DDiveFileType, 
    FileOp, 
    parse_directory_into_tree
};

use serde::Deserialize;
use std::collections::HashMap;



pub struct ImageLayer {
    pub name: String,
    pub tree: TreeNode,
    pub command: String,
}

impl ImageLayer {
    pub fn new(name: String, tree: TreeNode, command: String) -> ImageLayer {
        ImageLayer {
            name,
            tree,
            command,
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

    fn get_layer_path_cmd_wstr(layer : &str) -> PathBuf {
        ImageLayer::get_cache_dir().join(layer).with_extension("cmd")
    }

    fn get_layer_path_cmd(&self) -> PathBuf {
        ImageLayer::get_layer_path_cmd_wstr(&self.name)
    }

    pub fn save(&self) {
        // Serialise tree to json
        let layer_cache_path = self.get_layer_path();
        let mut layer_cache_file = File::create(&layer_cache_path).expect("Can't create layer cache file");
        let layer_json = serde_json::to_string(&self.tree).expect("Can't serialize layer tree");
        layer_cache_file.write_all(layer_json.as_bytes()).expect("Can't write to layer cache file");        
        // Serialise command to file
        let cmd_cache_file = self.get_layer_path_cmd();
        let mut cmd_cache_file = File::create(&cmd_cache_file).expect("Can't create command cache file");
        cmd_cache_file.write_all(self.command.as_bytes()).expect("Can't write to command cache file");
    }

    pub fn load(layer : &str) -> Result<ImageLayer, Box<dyn std::error::Error> > {
        let layer_cache_path = ImageLayer::get_layer_path_wstr(layer);
        let layer_cache_file = File::open(&layer_cache_path)?;
        let layer_tree : TreeNode = serde_json::from_reader(layer_cache_file)?;
        let cmd_cache_file = ImageLayer::get_layer_path_cmd_wstr(layer);
        let mut cmd_cache_file = File::open(&cmd_cache_file)?;
        let mut command = String::new();
        cmd_cache_file.read_to_string(&mut command)?;

        Ok(ImageLayer::new(layer.to_string(), layer_tree, command))
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
    #[allow(dead_code)]
    name : String,
    #[allow(dead_code)]
    temp_dir : TempDir,
    pub layers : Vec<ImageLayer>,
}

impl ImageRepr {
    pub async fn new(name: String, docker: &Docker) -> ImageRepr {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let img_tar_file_path = PathBuf::from(temp_dir.path()).join("image.tar");
        let img_folder = PathBuf::from(temp_dir.path()).join("image");

        let layers = get_image_layers(&docker, &name).await;
        let all_layers_cached: bool = layers.iter().all(|layer| ImageLayer::check_cache(layer));

        if all_layers_cached {
            let mut all_layers : Vec<ImageLayer> = Vec::new();
            for layer in layers {
                let layer = ImageLayer::load(&layer).expect("Can't load cached layer");
                all_layers.push(layer);
            }
            return ImageRepr {
                name: name,
                temp_dir: temp_dir,
                layers: all_layers,
            }
        }

        println!("Missing some layers in the cache, need to redownload the image");

        download_image_file(&docker, &name, &img_tar_file_path).await;

        // Untar image
        let file: File = File::open(&img_tar_file_path).expect("Can't open file");
        let mut archive = Archive::new(file);
        archive.unpack(&img_folder).expect("Can't unpack image");

        println!("Finished downloading the image, unpacking layers");

        let layer_folder = img_folder.join("blobs").join("sha256");

        // split layers into cached and non-cached
        let mut non_cached_layers = layers.clone();

        // Get cached layers
        let _ = ImageLayer::filter_cached_layers(&mut non_cached_layers);

        // get non-cached layers
        let layer_trees: Vec<(String, TreeNode)> = unpack_image_layers(&layer_folder, &non_cached_layers);
        let manifest_file = get_manifest_config_file(&img_folder);
        let mut commands = get_layer_commands(&img_folder, &manifest_file);
        
        if commands.len() != layer_trees.len() {
            print!("Commands and layers size mismatch; commands won't be printed");
            commands.clear();
            commands.resize(layer_trees.len(), String::from("Not available"));

        }


        // Construct cache from non-cached layers
        for ((layer_name, layer_tree), command) in layer_trees.into_iter().zip(commands.into_iter()) {
            let layer = ImageLayer::new(layer_name, layer_tree, command);
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

#[allow(dead_code)]
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

pub async fn get_image_layers(docker: &Docker, img_name: &String) -> Vec<String> {
    let image_details = docker.inspect_image(img_name).await.unwrap();
    let rfs = image_details.root_fs.expect("No fs");
    let layers = rfs.layers.expect("No layers in the image");
    layers
}

pub async fn download_image_file(docker: &Docker, img_name: &String, img_tar_file_path : &PathBuf) {
    let mut stream = docker.export_image(img_name);
    let mut context = std::task::Context::from_waker(futures_util::task::noop_waker_ref());
    let mut file = File::create(&img_tar_file_path).expect("Can't create file");


    loop { 
        let poll_res = stream.poll_next_unpin(&mut context); 
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
        let layer_tree = layer_tree.prettyfy();
        // Change root of the tree to the first child
        layer_trees.push((layer.to_string(), layer_tree));
    }

    layer_trees
}

pub fn get_manifest_config_file(docker_root_folder: &PathBuf) -> String  {
    let manifest_path = docker_root_folder.join("manifest.json");
    let manifest_file = File::open(&manifest_path).expect("Can't open manifest file");
    let manifest : Vec<Manifest> = serde_json::from_reader(manifest_file).expect("Can't parse manifest file");
    if manifest.len() != 1 {
        panic!("Can't parse manifest file, expected 1 image, got {}", manifest.len());
    }
    let config_file = &manifest[0].Config;
    config_file.clone()
}

pub fn get_layer_commands(docker_root_folder: &PathBuf, config_file : &str) -> Vec<String> {
    let config_path = docker_root_folder.join(config_file);
    let config_file = File::open(&config_path).expect("Can't open config file");
    let config : Config = serde_json::from_reader(config_file).expect("Can't parse config file");
    let mut commands : Vec<String> = Vec::new();
    for history in config.history {
        match history.empty_layer {
            Some(empty) => { 
                if empty {
                    continue;
                } else {
                    commands.push(history.created_by.trim().to_string());
                }
            },
            None => {
                commands.push(history.created_by.trim().to_string());
            }
        }
    }

    commands
}

#[allow(dead_code, non_snake_case)]
#[derive(Debug, Deserialize)]
struct Manifest {
    Config: String,
    RepoTags: Option<Vec<String>>,
    Layers: Vec<String>,
    LayerSources: Option<HashMap<String, LayerSource>>,
}

#[allow(dead_code, non_snake_case)]
#[derive(Debug, Deserialize)]
struct LayerSource {
    mediaType: String,
    size: u64,
    digest: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    history: Vec<History>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct History {
    created: String,
    created_by: String,
    empty_layer: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    

    const DOCKER_FOLDER_PATH: &str = "test-docker-tar/";

    #[test]
    fn read_manifest_file() {
        let manifest_path = PathBuf::from(DOCKER_FOLDER_PATH).join("manifest.json");
        let manifest_file = File::open(&manifest_path).expect("Can't open manifest file");
        let manifest : Vec<Manifest> = serde_json::from_reader(manifest_file).expect("Can't parse manifest file");
        println!("{:?}", manifest);
    }

    #[test]
    fn get_config_file_from_manifest() {
        let docker_root_folder = PathBuf::from(DOCKER_FOLDER_PATH);
        let config_file = get_manifest_config_file(&docker_root_folder);
        assert_eq!(config_file, "blobs/sha256/0d99781172fa4757fb472183792b0d6e1df6d180d6361ea0ae5872ee4adc1f1c");
    }

    #[test]
    fn read_config_file() {
        let docker_root_folder = PathBuf::from(DOCKER_FOLDER_PATH);
        let config_file = get_manifest_config_file(&docker_root_folder);
        let commands = get_layer_commands(&docker_root_folder, &config_file);
        print!("{:?}", commands);
        assert_eq!(commands.len(), 5);
        assert_eq!(commands[0], "/bin/sh -c #(nop) ADD file:1b6c8c9518be42fa2afe5e241ca31677fce58d27cdfa88baa91a65a259be3637 in /");
        assert_eq!(commands[1], "RUN /bin/sh -c mkdir -p /home/georgii # buildkit");
        assert_eq!(commands[2], "RUN /bin/sh -c echo \"Hello, Georgii!\" > /home/georgii/hello.txt # buildkit");
        assert_eq!(commands[3], "RUN /bin/sh -c echo \"Hello, Georgii2!\" > /home/georgii/hello2.txt # buildkit");
        assert_eq!(commands[4], "RUN /bin/sh -c rm /home/georgii/hello.txt # buildkit");
    }



}