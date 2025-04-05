use crate::docker_file_tree::{FileOp, TreeNode};
use crate::docker_image_utils::{
    ImageLayer, 
    ImageRepr
};
use crate::widgets::tree_browser_widget::{TreeBrowserWidget, TreeBrowserWidgetState};
use std::{
    collections::HashMap, 
    time::Duration, 
};
use std::io;
use ratatui::text::Text;
use ratatui::widgets::{Clear, Paragraph, StatefulWidget};
use ratatui::{
    layout::{
        Constraint, 
        Direction, 
        Layout
    }, 
    style::{
        Color, 
        Modifier, 
        Style
    }, 
    text::Span, 
    widgets::{
        Block, 
        Borders, 
        List, 
        ListItem, 
        ListState, 
    }, 
    DefaultTerminal, 
    Frame
};
use crossterm::event::{
    self, 
    Event, 
    KeyCode, 
    KeyEvent
};

use tui_tree_widget::{
    Tree, 
    TreeItem, 
    TreeState
};

use ratatui::prelude::Widget;

use crate::widgets::layer_browser_widget::LayerBrowserWidget;

enum Focus {
    List,
    Tree,
    SearchBar,
}

fn next_list_state(state : &mut ListState, size : usize) {
    if let Some(selected) = state.selected() {
        if selected < size - 1 {
            state.select(Some(selected + 1));
        }
    }
}

fn prev_list_state(state : &mut ListState) {
    if let Some(selected) = state.selected() {
        if selected > 0 {
            state.select(Some(selected - 1));
        }
    }
}

pub struct App {
    item: ImageRepr,
    exit: bool,
    tree_state: TreeBrowserWidgetState,
    list_state: ListState,
    layer_names: Vec<String>,
    layer_commands: Vec<String>,
    n_layers: usize,
    focus: Focus,
    search_bar_content: String,
}

impl App {
    pub fn new(item: ImageRepr) -> App {
        let layer_names : Vec<String> = App::layer_names_from_img(&item);
        let layer_commands : Vec<String> = item.layers.iter().map(|layer| layer.command.clone()).collect();
        if layer_names.len() == 0 {
            panic!("No layers found in image");
        }
        if layer_names.len() != layer_commands.len() {
            panic!("Layer names and commands are not the same length");
        }
        let n_layers = layer_names.len();

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        App { 
            item,
            exit: false,
            layer_names,
            layer_commands,
            n_layers,
            list_state,
            focus: Focus::List,
            search_bar_content: "".to_string(),
            tree_state: TreeBrowserWidgetState::new(""),
        }

    }

    fn layer_names_from_img(img : &ImageRepr) -> Vec<String> {
        img.layers.iter().map(|layer| layer.name.clone()).collect()
    }

    fn layer_names(&self) -> Vec<String> {
        self.item.layers.iter().map(|layer| layer.name.clone()).collect()
    }

    
    // fn next_tree(&mut self) {
    //     self.tree_state.select_relative(|current| {
    //         current.map_or(0, |current| current.saturating_add(1))
    //     });
    // }

    fn circle_focus(&mut self) {
        match self.focus {
            Focus::List => self.focus = Focus::Tree,
            Focus::Tree => self.focus = Focus::List,
            Focus::SearchBar => {}
        }
    }

    fn next(&mut self) {
        match self.focus {
            
            Focus::List => next_list_state(&mut self.list_state, self.n_layers),
            // Focus::Tree => self.next_tree(),
            Focus::Tree => {},
            Focus::SearchBar => {}
        }
    }

    // fn previous_tree(&mut self) {
    //     self.tree_state.select_relative(|current| {
    //         current.map_or(0, |current| current.saturating_sub(1))
    //     });
    // }
    
    fn previous(&mut self) {
        match self.focus {
            Focus::List => prev_list_state(&mut self.list_state),
            // Focus::Tree => self.previous_tree(),
            Focus::Tree => {},
            Focus::SearchBar => {}
        }
    }

    // fn expand_tree(&mut self) {
    //     self.tree_state.toggle_selected();
    // }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render(& mut self, frame: &mut Frame) {
        let area  = frame.area();
        let vlayout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(95), Constraint::Percentage(5)].as_ref())
            .split(area);

        let hlayout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(vlayout[0]);

        // let items: Vec<ListItem> = self
        //     .layer_names()
        //     .iter()
        //     .map(|i| ListItem::new(Span::from(i.clone())))
        //     .collect();

        let list_title = match self.focus {
            Focus::List => "😍 Layers ",
            _ => "Layers",
        };

        let tree_title = match self.focus {
            Focus::Tree => "😍 Tree",
            _ => "Tree",
        };

        let search_title = match self.focus {
            Focus::SearchBar => "😍 Search",
            _ => "Search",
        };

        // let list = List::new(items)
        // .block(Block::default().borders(Borders::ALL).title(list_title))
        // .highlight_style(
        //     Style::default()
        //         .bg(Color::Blue)
        //         .fg(Color::White)
        //         .add_modifier(Modifier::BOLD),
        // )
        // .highlight_symbol(">> ");

        let layers_and_commands = LayerBrowserWidget::new(&self.layer_names, &self.layer_commands);

        frame.render_stateful_widget(layers_and_commands, hlayout[0], &mut self.list_state);

        // Test tree widget
        let tree_widget = TreeBrowserWidget::new(&self.item.layers[0]);
        frame.render_stateful_widget(tree_widget, hlayout[1], &mut self.tree_state);

        // let text = App::split_string_into_vec(self.get_layer_command(), vlayout_left[1].width as usize - 10);
        // let command = List::new(text)
        // .block(Block::default().borders(Borders::ALL).title("Command"));



        // let current_layer = &self.item.layers[self.selected_layer];

        // if self.tree_state.selected().len() == 0 {
        //     self.tree_state.select_first();
        // }



        // frame.render_stateful_widget(tree_widget, hlayout[1], & mut self.tree_state);

        // let search = Paragraph::new(self.search_bar_content.clone())
        //     .block(Block::default()
        //     .borders(Borders::ALL)
        //     .title(search_title));

        // frame.render_widget(search, vlayout[1]);
        // // frame.render_widget(Clear, frame.area());

    }

    fn split_string_into_size(s: &str, size: usize) -> String {
        let mut res = String::default();
        if (s.len() <= size) {
            return s.to_string();
        }
        let mut current_pos = 0;
        while current_pos < s.len() - size {
            res.push_str(&s[current_pos..current_pos + size]);
            res.push('\n');
            current_pos += size;
        }

        res.push_str(&s[current_pos..s.len()]);
        return res;
    }

    fn split_string_into_vec(s: &str, size: usize) -> Vec<String> {
        let mut res = Vec::new();
        let mut current_pos = 0;
        let mut s = s.to_string();
        s.retain(|c| !c.is_control());

        if (s.len() <= size) {
            return vec![s.to_string()];
        }
        while current_pos < s.len() - size {
            res.push(s[current_pos..current_pos + size].to_string());
            current_pos += size;
        }

        res.push(s[current_pos..s.len()].to_string());
        return res;
    }

    fn remove_control_chars_from_string(s: &str) -> String {
        let mut res = String::default();
        for c in s.chars() {
            if !c.is_control() {
                res.push(c);
            }
        }
        res
    }

    fn get_all_key_events() -> Vec<KeyEvent> {
        let mut key_events = Vec::new();
        loop {
            let event = event::poll(Duration::from_millis(0));
            if event.is_err() {
                break;
            }
            let event = event.unwrap();
            if ! event {
                break;
            } else {
                let event = event::read().expect("Can't read key event");
                if let Event::Key(key_event) = event {
                    key_events.push(key_event);
                }
            }
        }
        key_events
    }

    fn handle_events(&mut self) -> io::Result<()> {
        let key_events = App::get_all_key_events();
        for key_event in key_events {
            match self.focus {
                Focus::SearchBar => {
                    match key_event.code {
                        KeyCode::Char(c) => {
                            self.search_bar_content.push(c);
                        }
                        KeyCode::Backspace => {
                            self.search_bar_content.pop();
                        }
                        KeyCode::Enter => {
                            self.focus = Focus::Tree;
                        }
                        KeyCode::Esc => {
                            self.focus = Focus::Tree;
                        }
                        _ => {}
                    }
                },
                _ => {
                    match key_event.code {
                        KeyCode::Down => self.next(), // Move selection down
                        KeyCode::Up => self.previous(), // Move selection up
                        KeyCode::Tab => self.circle_focus(), // Switch between list and tree
                        // KeyCode::Char(' ') => self.expand_tree(), // Expand tree
                        KeyCode::Char('q') => self.exit = true, // Quit
                        KeyCode::Char('f') if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => self.focus = Focus::SearchBar,
                    _ => {}
                }
            }
            }
            
        }
        Ok(())
    }
}