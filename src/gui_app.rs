use crate::docker_file_tree::TreeNode;
use crate::docker_image_utils::{
    ImageLayer, 
    ImageRepr
};
use std::{
    collections::HashMap, 
    time::Duration, 
};
use std::io;
use ratatui::widgets::Paragraph;
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

enum Focus {
    List,
    Tree,
    SearchBar,
}

pub struct App {
    item: ImageRepr,
    selected_layer: usize,
    exit: bool,
    list_state: ListState,
    tree_state: TreeState<String>,
    focus: Focus,
    search_bar_content: String,
}

impl App {
    pub fn new(item: ImageRepr) -> App {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let tree_state: TreeState<String> = TreeState::default();
        App { 
            item, 
            selected_layer: 0, 
            exit: false, 
            list_state, 
            tree_state, 
            focus: Focus::List,
            search_bar_content: "".to_string(),
        }
    }

    fn layer_names(&self) -> Vec<String> {
        self.item.layers.iter().map(|layer| layer.name.clone()).collect()
    }

    fn next_list(&mut self) {
        if self.selected_layer < self.layer_names().len() - 1 {
            self.selected_layer += 1;
            self.list_state.select(Some(self.selected_layer));
        }
    }

    fn construct_items<'a>(layer : &'a ImageLayer, filter_str: &'a str) -> Vec<TreeItem<'a, String> > {
        let tree = &layer.tree;
        let filtered_tree = tree.filter_tree_full_path(filter_str);

        let tree = if let Some(filt_tree) = filtered_tree.as_ref() {
            filt_tree // Reference to the filtered tree
        } else {
            &layer.tree // Reference to the original tree
        };
    
        // parents at the start, children at the end
        let nodes_vec : Vec<&TreeNode> = tree.breadth_first();

        let mut map: HashMap<&TreeNode, TreeItem<String> > = HashMap::new();

        for &node in nodes_vec.iter().rev() {
            if node == tree { // Skip the root
                break;
            }
            let try_name  = node.path().file_name();
            let mut name : String = "root".to_string();
            match try_name {
                Some(try_name) => {
                    name = try_name.to_str().expect("WTF").to_string();
                },
                None => {
                }
            }
                    
            let path = String::from(node.path().to_str().expect("WTF"));
            // let name  = String::from(node.path().file_name().expect("WTF").to_str().expect("WTF"));

            if node.kids().len() == 0 {
                let leaf = TreeItem::new_leaf(path.clone(), name.clone());
                map.insert(node, leaf);
            } else {               
                let kids = node.kids();
                // Because we are iterating in the reverse order of breadth-first search, we can assume that the children are already in the map
                let mut kids_items : Vec<TreeItem<String> > = Vec::new();
                for kid in kids {
                    let kid_item : TreeItem<String> = map.remove(kid).expect("Can't find child in map");
                    kids_items.push(kid_item);
                }
                let tree_item = TreeItem::new(path.clone(), name.clone(), kids_items).expect("Can't create tree item");
                map.insert(node, tree_item);
            }
        }

        // All that should be left in the map are the nodes below the root

        let keys : Vec<TreeItem<String> > = map.into_values().collect();
        // sort by name
        let mut keys = keys;
        keys.sort_by(|a, b| a.identifier().cmp(b.identifier()));
        keys
    }


    fn next_tree(&mut self) {
        self.tree_state.select_relative(|current| {
            current.map_or(0, |current| current.saturating_add(1))
        });
    }

    fn circle_focus(&mut self) {
        match self.focus {
            Focus::List => self.focus = Focus::Tree,
            Focus::Tree => self.focus = Focus::List,
            Focus::SearchBar => {}
        }
    }

    fn next(&mut self) {
        match self.focus {
            Focus::List => self.next_list(),
            Focus::Tree => self.next_tree(),
            Focus::SearchBar => {}
        }
    }

    fn previous_list(&mut self) {
        if self.selected_layer > 0 {
            self.selected_layer -= 1;
            self.list_state.select(Some(self.selected_layer));
        }
    }

    fn previous_tree(&mut self) {
        self.tree_state.select_relative(|current| {
            current.map_or(0, |current| current.saturating_sub(1))
        });
    }

    fn previous(&mut self) {
        match self.focus {
            Focus::List => self.previous_list(),
            Focus::Tree => self.previous_tree(),
            Focus::SearchBar => {}
        }
    }

    fn expand_tree(&mut self) {
        self.tree_state.toggle_selected();
    }

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

        let items: Vec<ListItem> = self
            .layer_names()
            .iter()
            .map(|i| ListItem::new(Span::from(i.clone())))
            .collect();

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

        let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        frame.render_stateful_widget(list, hlayout[0], &mut self.list_state);
        let current_layer = &self.item.layers[self.selected_layer];
        let items = App::construct_items(current_layer, &self.search_bar_content);

        if self.tree_state.selected().len() == 0 {
            self.tree_state.select_first();
        }

        let tree_widget = Tree::new(&items).expect("WTF")
        .block(Block::default().borders(Borders::ALL).title(tree_title))
        .highlight_style(
                Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        frame.render_stateful_widget(tree_widget, hlayout[1], & mut self.tree_state);

        let search = Paragraph::new(self.search_bar_content.clone())
            .block(Block::default()
            .borders(Borders::ALL)
            .title(search_title));

        frame.render_widget(search, vlayout[1]);
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
                        KeyCode::Char(' ') => self.expand_tree(), // Expand tree
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