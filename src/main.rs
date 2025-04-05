mod docker_file_tree;
mod docker_image_utils;
mod widgets;
mod gui_app;
use docker_image_utils::ImageRepr;
use gui_app::App;
use bollard::Docker;
use std::error::Error;
use clap::{
    command,Arg
};


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error> >{
    // let args = Args::parse();
    let matches = command!() // requires `cargo` feature
    .arg(Arg::new("Name"))
    .get_matches();

    let img_name : String = matches.get_one::<String>("Name").expect("Can't parse to string").clone();

    let docker = Docker::connect_with_socket_defaults().expect("Can't connect to docker");

    let img = ImageRepr::new(img_name, &docker).await;
    
    let mut terminal = ratatui::init();

    

    let mut app = App::new(img);
    let _ = app.run(&mut terminal);

    ratatui::restore();


    Ok(())
}
