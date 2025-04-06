use std::collections::HashMap;

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, Paragraph, StatefulWidget, Widget};
use tui_tree_widget::{Tree, TreeItem, TreeState};
use crate::docker_file_tree::{FileOp, TreeNode};
use crate::docker_image_utils::ImageLayer;

use super::navigation_traits::WidgetNav;

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
            None => {}
        }
                
        let path = String::from(node.path().to_str().expect("WTF"));
        // let name  = String::from(node.path().file_name().expect("WTF").to_str().expect("WTF"));

        let name: Text = match node.fop() {
            FileOp::Add => {
                let style = Style::new().fg(Color::Green);
                Text::styled(name, style)
            },
            FileOp::Remove => {
                let style = Style::new().fg(Color::Red);
                Text::styled(name, style)
            },
        };

        if node.kids().len() == 0 {
            let leaf = TreeItem::new_leaf(path.clone(), name);
            map.insert(node, leaf);
        } else {               
            let kids = node.kids();
            // Because we are iterating in the reverse order of breadth-first search, we can assume that the children are already in the map
            let mut kids_items : Vec<TreeItem<String> > = Vec::new();
            for kid in kids {
                let kid_item : TreeItem<String> = map.remove(kid).expect("Can't find child in map");
                kids_items.push(kid_item);
            }
            let tree_item = TreeItem::new(path.clone(), name, kids_items).expect("Can't create tree item");
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


/// A widget that displays a tree structure using searchbar state; This does not correspond to multiple strings
pub struct TreeBrowserWidgetState {
    search_string: String,
    tree_state: TreeState<String>,
}

impl TreeBrowserWidgetState {
    pub fn new(search_string: &str) -> Self {
        TreeBrowserWidgetState {
            search_string: search_string.to_string(),
            tree_state: TreeState::default(),
        }
    }

    pub fn expand(&mut self) {
        let mut selected_state = &mut self.tree_state;
        selected_state.toggle_selected();
    }

    pub fn set_search_string(&mut self, search_string: &str) {
        self.search_string = search_string.to_string();
    }
}

pub struct TreeBrowserWidget<'a> {
    corresponding_layer: &'a ImageLayer,
}

impl<'a> TreeBrowserWidget<'a> {
    pub fn new(layer: &'a ImageLayer) -> Self {
        TreeBrowserWidget {
            corresponding_layer: layer,
        }
    }
}

impl WidgetNav for TreeBrowserWidgetState {
    fn next(&mut self) {
        let mut selected_state = &mut self.tree_state;
        selected_state.select_relative(|current| {
            current.map_or(0, |current| current.saturating_add(1))
        });
    }

    fn prev(&mut self) {
        let mut selected_state = &mut self.tree_state;
        selected_state.select_relative(|current| {
            current.map_or(0, |current| current.saturating_sub(1))
        });
    }
}


impl<'a> StatefulWidget for TreeBrowserWidget<'a> {
    type State = TreeBrowserWidgetState;

    fn render(self, area: Rect, buf: & mut Buffer, state: & mut Self::State){ 
        let items = construct_items(self.corresponding_layer, &state.search_string);

        let tree_widget = Tree::new(&items).expect("WTF")
        .highlight_style(
                Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        StatefulWidget::render(tree_widget, area, buf, &mut state.tree_state);
        
    }
}