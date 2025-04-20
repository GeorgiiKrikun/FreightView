mod docker_file_tree;
mod docker_image_utils;
mod exceptions;
mod gui_app;
mod widgets;
use bollard::Docker;
use clap::{Arg, command};
use docker_image_utils::ImageRepr;
use gui_app::App;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!().arg(Arg::new("Name")).get_matches();

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
