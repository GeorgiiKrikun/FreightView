use super::navigation_traits::WidgetNav;
use crate::docker_image_utils::ImageLayer;
use crate::exceptions::GUIError;
use crate::file_tree::FileTreeNode;
use crate::file_tree::{EntryOp, EntryType};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::StatefulWidget;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use tui_tree_widget::{Tree, TreeItem, TreeState};

fn construct_items<'a>(
    layer: &'a ImageLayer,
    filter_str: &'a str,
) -> (Vec<TreeItem<'a, PathBuf>>, Option<GUIError>) {
    // TODO:: think why this compiles without reference
    let tree = &layer.tree;
    // let filtered_tree = tree.filter_tree_full_path(filter_str);
    // todo!("Implement the filter_tree_full_path function");
    // let tree = if let Some(filt_tree) = filtered_tree.as_ref() {
    //     filt_tree // Reference to the filtered tree
    // } else {
    //     &layer.tree // Reference to the original tree
    // };
    let (tree, error) = tree.filter_tree_full_path(filter_str);

    // parents at the start, children at the end
    let nodes_vec: Vec<Rc<RefCell<FileTreeNode>>> = tree.iter().collect();
    let root = tree.root();
    let mut map: HashMap<PathBuf, TreeItem<PathBuf>> = HashMap::new();

    for node in nodes_vec.iter().rev() {
        if node.borrow().name() == root.borrow().name() {
            // Skip the root
            break;
        }
        let name = node.borrow().name();
        let path = node.borrow().path();
        let name = match node.borrow().ftype() {
            EntryType::Directory => name,
            EntryType::File => name,
            EntryType::Symlink(points_to) => name + " -> " + points_to.to_string_lossy().as_ref(),
            EntryType::Badfile => name + " (invalid)",
        };

        let name: Text = match node.borrow().fop() {
            EntryOp::Add => {
                let style = Style::new().fg(Color::Green);
                Text::styled(name, style)
            }
            EntryOp::Remove => {
                let style = Style::new().fg(Color::Red);
                Text::styled(name, style)
            }
        };

        if node.borrow().get_n_children() == 0 {
            let leaf = TreeItem::new_leaf(path.clone(), name);
            map.insert(path.clone(), leaf);
        } else {
            let kids_paths = node.borrow().get_children_paths();
            // Because we are iterating in the reverse order of breadth-first search, we can assume that the children are already in the map
            let mut kids_items: Vec<TreeItem<PathBuf>> = Vec::new();
            for kid in kids_paths.iter() {
                let kid_item = map.remove(kid);
                match kid_item {
                    Some(item) => {
                        kids_items.push(item);
                    }
                    None => {
                        unreachable!("Kid item that has been in the mapn is not found");
                    }
                }
            }
            let tree_item =
                TreeItem::new(path.clone(), name, kids_items).expect("Can't create tree item");
            map.insert(path, tree_item);
        }
    }

    // All that should be left in the map are the nodes below the root

    let keys: Vec<TreeItem<PathBuf>> = map.into_values().collect();
    // sort by name to avoid tree jumping in the browser
    let mut keys = keys;
    keys.sort_by(|a, b| a.identifier().cmp(b.identifier()));
    (keys, error)
}

/// A widget that displays a tree structure using searchbar state; This does not correspond to multiple strings
pub struct TreeBrowserWidgetState {
    search_string: String,
    search_error: bool,
    tree_state: TreeState<PathBuf>,
}

impl TreeBrowserWidgetState {
    pub fn new(search_string: &str) -> Self {
        TreeBrowserWidgetState {
            search_string: search_string.to_string(),
            search_error: false,
            tree_state: TreeState::default(),
        }
    }

    pub fn expand(&mut self) {
        let selected_state = &mut self.tree_state;
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

fn select_next_tree_state(tree_state: &mut TreeState<PathBuf>) {
    tree_state.select_relative(|current| current.map_or(0, |current| current.saturating_add(1)));
}

fn select_prev_tree_state(tree_state: &mut TreeState<PathBuf>) {
    tree_state.select_relative(|current| current.map_or(0, |current| current.saturating_sub(1)));
}

impl WidgetNav for TreeBrowserWidgetState {
    fn next(&mut self) {
        select_next_tree_state(&mut self.tree_state);
    }

    fn prev(&mut self) {
        select_prev_tree_state(&mut self.tree_state);
    }
}

impl<'a> StatefulWidget for TreeBrowserWidget<'a> {
    type State = TreeBrowserWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (items, error) = construct_items(self.corresponding_layer, &state.search_string);
        match error {
            Some(GUIError::CantFilterTree) => {
                state.search_error = true;
            }
            None => {
                state.search_error = false;
            }
        }

        let mut tree_widget = Tree::new(&items)
            .expect("WTF")
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        if state.search_error {
            tree_widget = tree_widget.style(Style::default().bg(Color::Red));
        }

        StatefulWidget::render(tree_widget, area, buf, &mut state.tree_state);
    }
}
