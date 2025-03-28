mod docker_file_tree;
mod docker_image_utils;
use docker_image_utils::ImageRepr;
use bollard::Docker;
use std::error::Error;
use clap::{command,Arg};
use std::io;
use ratatui::{
    backend::CrosstermBackend, buffer::Buffer, layout::{self, Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::Span, widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget}, DefaultTerminal, Frame, Terminal
};
use crossterm::event::{self, Event, KeyCode};
use tui_tree_widget::{Tree, TreeItem, TreeState};

struct App {
    item: ImageRepr,
    selected_layer: usize,
    layer_names: Vec<String>,
    exit: bool,
    list_state: ListState,
}

impl App {
    fn new(item: ImageRepr) -> App {
        let layers_names : Vec<String> = item.layers.iter().map(|layer| layer.name.clone()).collect();
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        App { item : item, selected_layer: 0, layer_names : layers_names, exit: false, list_state: list_state }
    }

    fn next(&mut self) {
        if self.selected_layer < self.layer_names.len() - 1 {
            self.selected_layer += 1;
            self.list_state.select(Some(self.selected_layer));
        }
    }

    fn previous(&mut self) {
        if self.selected_layer > 0 {
            self.selected_layer -= 1;
            self.list_state.select(Some(self.selected_layer));
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Down => self.next(), // Move selection down
                KeyCode::Up => self.previous(), // Move selection up
                KeyCode::Char('q') => self.exit = true, // Quit
                _ => {}
            }
        }
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hlayout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let items: Vec<ListItem> = self
            .layer_names
            .iter()
            .map(|i| ListItem::new(Span::from(i.clone())))
            .collect();


        let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Select an Item"))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        StatefulWidget::render(list, hlayout[0], buf, &mut self.list_state.clone());
        let a = TreeItem::new_leaf("l", "Leaf");
        let b = TreeItem::new("r", "Root", vec![a]).expect("WHAT");
        let items = vec![b];

        let tree_widget = Tree::new(&items).expect("WTF");
        let mut tree_state : TreeState<&str> = TreeState::default();

        StatefulWidget::render(tree_widget, hlayout[1], buf, &mut tree_state);


    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error> >{
    // let args = Args::parse();
    let matches = command!() // requires `cargo` feature
    .arg(Arg::new("Name"))
    .get_matches();

    let img_name : String = matches.get_one::<String>("Name").expect("Can't parse to string").clone();

    let docker = Docker::connect_with_socket_defaults().expect("Can't connect to docker");

    let img = ImageRepr::new(img_name, &docker).await;
    
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::init();

    

    let mut app = App::new(img);
    let res = app.run(&mut terminal);

    ratatui::restore();


    Ok(())
}
