mod docker_image_utils;
mod exceptions;
mod file_tree;
mod gui_app;
mod widgets;
use bollard::Docker;
use clap::{Arg, command};
use docker_image_utils::ImageRepr;
use gui_app::App;
use std::error::Error;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!().arg(Arg::new("Name")).get_matches();

    let img_name: String = matches
        .get_one::<String>("Name")
        .expect("Can't parse to string")
        .clone();

    let docker = Docker::connect_with_socket_defaults().expect("Can't connect to docker");

    let start = Instant::now();
    let img = ImageRepr::new(img_name, &docker).await;
    let cleanup_result = ImageRepr::clean_up_img_cache();
    match cleanup_result {
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "Error cleaning up image cache: {}, please cleanup manually, otherwise large cache will stay on your hard drive",
                e
            );
            return Err(Box::from(e));
        }
    }

    let elapsed = start.elapsed();
    println!("Startup time: {:?}", elapsed);

    let img = match img {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Error: {}", e);
            return Err(Box::from(e));
        }
    };

    let mut terminal = ratatui::init();

    let mut app = App::new(img);
    let _ = app.run(&mut terminal);

    ratatui::restore();

    Ok(())
}
