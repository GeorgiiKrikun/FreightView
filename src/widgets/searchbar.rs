

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListState, Paragraph, StatefulWidget, Widget};
use crate::widgets::navigation_traits::{WidgetNav, WidgetNavBounds};

use super::focus_traits::WidgetFocusTrait;

pub struct SearchBarWidgetState {
    title: String,
    is_toggled: bool,
    search_string: String,
}

pub struct SearchBarWidget {

}

impl SearchBarWidget {
    pub fn new() -> Self {
        Self {}
    }
}

impl SearchBarWidgetState {
    pub fn new() -> Self {
        SearchBarWidgetState {
            title: "Search".to_string(),
            is_toggled: false,
            search_string: "".to_string(),
        }
    }

    pub fn push_c(&mut self, ch: char) {
        self.search_string.push(ch);
    }
    
    pub fn pop_c(&mut self) {
        self.search_string.pop();
    }

    pub fn get(&self) -> String {
        self.search_string.clone()
    }
}

impl WidgetFocusTrait for SearchBarWidgetState {
    fn focus_on(&mut self, selected: bool) {
        self.is_toggled = selected;
        if self.is_toggled {
            self.title = "😍 Search Bar".to_string();
        } else {
            self.title = "Search Bar".to_string();
        }
    }

    fn is_focused(&self) -> bool {
        self.is_toggled
    }

}

impl StatefulWidget for SearchBarWidget {
    type State = SearchBarWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State){ 
        let search = Paragraph::new(state.search_string.clone())
        .block(
            Block::default()
            .borders(Borders::ALL)
            .title(state.title.clone())
        );

        Widget::render(search, area, buf);
    }
}