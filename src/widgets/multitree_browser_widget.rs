use std::collections::HashMap;

use ratatui::{buffer::Buffer, layout::Rect, widgets::StatefulWidget};
use tui_tree_widget::TreeState;

use crate::docker_image_utils::ImageLayer;
use crate::widgets::tree_browser_widget::{TreeBrowserWidget, TreeBrowserWidgetState};

pub struct MultiTreeBrowserWidgetState {
    pub search_string: String,
    pub current_layer: String,
    tree_states: HashMap<String, TreeBrowserWidgetState >,
}

impl MultiTreeBrowserWidgetState {
    pub fn new(search_string: &str, layer_names: &[String]) -> Self {
        let mut tree_states: HashMap<String, TreeBrowserWidgetState > = HashMap::new();
        for layer_name in layer_names {
            tree_states.insert(layer_name.to_string(), TreeBrowserWidgetState::new(search_string));
        }

        MultiTreeBrowserWidgetState {
            search_string: search_string.to_string(),
            tree_states: tree_states,
            current_layer: "".to_string(),
        }
    }
} 

pub struct MultiTreeBrowserWidget<'a> {
    tree_layers: HashMap<String, &'a ImageLayer >,
}

impl<'a> MultiTreeBrowserWidget<'a> {
    pub fn new(layers: &'a [ImageLayer]) -> Self {
        
        let mut map: HashMap<String, &'a ImageLayer > = HashMap::new();
        for layer in layers.iter() {
            let name = layer.name.clone();
            map.insert(name, layer);
        }

        MultiTreeBrowserWidget {
            tree_layers: map,
        }
    }
}

impl<'a> StatefulWidget for MultiTreeBrowserWidget<'a> {
    type State = MultiTreeBrowserWidgetState;

    fn render(self, area: Rect, buf: & mut Buffer, state: & mut Self::State) { 
        let cur_layer = &state.current_layer;
        if !self.tree_layers.keys().any(|k| k == cur_layer) {
            return;
        }
        let cur_tree_state = state.tree_states.get_mut(cur_layer).expect("Can't find current layer in state");
        let &cur_tree_layer = self.tree_layers.get(cur_layer).expect("Can't find current layer in widget");
        let tree_widget = TreeBrowserWidget::new(cur_tree_layer);

        StatefulWidget::render(tree_widget, area, buf, cur_tree_state);
    }
}