use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget, Widget};

use super::focus_traits::WidgetFocusTrait;

pub struct SearchBarWidgetState {
    title: String,
    is_toggled: bool,
    search_string: String,
}

pub struct SearchBarWidget {}

impl SearchBarWidget {
    pub fn new() -> Self {
        Self {}
    }

    // Simple logic for blinking - you might want something more sophisticated
    fn should_blink(&self) -> bool {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 1000
            < 500 // Blink every second, on for 500ms, off for 500ms
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

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let search_string = if state.is_toggled {
            if self.should_blink() {
                state.search_string.clone() + "█" // Add a cursor character at
            } else {
                state.search_string.clone()
            }
        } else {
            if state.search_string.is_empty() {
                "Filter here...".to_string()
            } else {
                state.search_string.clone()
            }
        };

        let search = Paragraph::new(search_string).block(
            Block::default()
                .borders(Borders::ALL)
                .title(state.title.clone()),
        );
        Widget::render(search, area, buf);
    }
}
