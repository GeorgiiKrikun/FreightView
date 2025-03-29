use crate::docker_file_tree::TreeNode;
use crate::docker_image_utils::{
    ImageLayer, 
    ImageRepr
};
use std::{
    collections::HashMap, 
    time::Duration, 
    vec
};
use std::io;
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


pub struct App {
    item: ImageRepr,
    selected_layer: usize,
    exit: bool,
    list_state: ListState,
    tree_state: TreeState<String>,
    list_selected: bool,
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
            list_selected: true, 
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

    fn construct_items(layer : &ImageLayer) -> Vec<TreeItem<String> > {
        let tree = &layer.tree;
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

        // let subroot_items_refs : Vec<&&TreeNode> = map.keys().collect();
        let huy : Vec<TreeItem<String> > = map.into_values().collect();
        // sort by name
        let mut huy = huy;
        huy.sort_by(|a, b| a.identifier().cmp(b.identifier()));
        huy




    }


    fn next_tree(&mut self) {
        self.tree_state.select_relative(|current| {
            current.map_or(0, |current| current.saturating_add(1))
        });
    }

    fn next(&mut self) {
        if self.list_selected {
            self.next_list();
        } else {
            self.next_tree();
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
        if self.list_selected {
            self.previous_list();
        } else {
            self.previous_tree();
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
        let hlayout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let items: Vec<ListItem> = self
            .layer_names()
            .iter()
            .map(|i| ListItem::new(Span::from(i.clone())))
            .collect();


        let (list_title, tree_title) = if self.list_selected {
            ("😍 Layers ", "Filesystem tree view")
        } else {
            ("Layers", "😍 Filesystem tree view ")
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
        // let a = TreeItem::new_leaf("l", "Leaf1");
        // let b = TreeItem::new("r", "Root", vec![a]).expect("WHAT");
        // let c = TreeItem::new_leaf("l", "Leaf2");
        // let d = TreeItem::new_leaf("heh", "Leaf3");
        // let items = vec![b,c,d];
        let current_layer = &self.item.layers[self.selected_layer];
        let items = App::construct_items(current_layer);

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
            match key_event.code {
                KeyCode::Down => self.next(), // Move selection down
                KeyCode::Up => self.previous(), // Move selection up
                KeyCode::Tab => self.list_selected = !self.list_selected, // Switch between list and tree
                KeyCode::Char(' ') => self.expand_tree(), // Expand tree
                KeyCode::Char('q') => self.exit = true, // Quit
                _ => {}
            }
        }
        Ok(())
    }
}