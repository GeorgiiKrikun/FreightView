mod docker_file_tree;
mod docker_image_utils;
mod exceptions;
mod file_tree;
mod file_tree_node;
mod gui_app;
mod widgets;
use bollard::Docker;
use clap::{Arg, command};
use config::Config;
use docker_image_utils::ImageRepr;
use gui_app::App;
use std::error::Error;

const APP_NAME: &str = "FreightView";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!().arg(Arg::new("Name")).get_matches();
    let path_to_config = match std::env::var("HOME") {
        Ok(path) => path + "/.config/" + APP_NAME + "/config.toml",
        Err(_) => "config.toml".to_string(),
    };

    let img_name: String = matches
        .get_one::<String>("Name")
        .expect("Can't parse to string")
        .clone();

    let docker = Docker::connect_with_socket_defaults().expect("Can't connect to docker");

    let img = ImageRepr::new(img_name, &docker).await?;

    let mut terminal = ratatui::init();

    let mut app = App::new(img);
    let _ = app.run(&mut terminal);

    ratatui::restore();

    Ok(())
}
