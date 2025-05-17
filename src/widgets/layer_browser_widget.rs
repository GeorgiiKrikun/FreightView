use crate::widgets::navigation_traits::{WidgetNav, WidgetNavBounds};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListState, Paragraph, StatefulWidget, Widget};

use super::focus_traits::WidgetFocusTrait;

pub struct LayerBrowserWidgetState {
    title: String,
    is_toggled: bool,
    pub state: ListState,
}

pub struct LayerBrowserWidget<'a> {
    layer_names: &'a Vec<String>,
    layer_commands: &'a Vec<String>,
}

impl<'a> LayerBrowserWidget<'a> {
    pub fn new(layer_names: &'a Vec<String>, layer_commands: &'a Vec<String>) -> Self {
        Self {
            layer_names,
            layer_commands,
        }
    }
}

impl LayerBrowserWidgetState {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        LayerBrowserWidgetState {
            title: "Layer Browser".to_string(),
            is_toggled: true,
            state,
        }
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.state.select(index);
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }
}

impl WidgetNav for LayerBrowserWidgetState {
    fn next(&mut self) {
        if let Some(selected) = self.selected() {
            self.select(Some(selected + 1));
        } else {
            self.select(Some(0));
        }
    }

    fn prev(&mut self) {
        if let Some(selected) = self.selected() {
            if selected > 0 {
                self.select(Some(selected - 1));
            }
        } else {
            self.select(Some(0));
        }
    }
}

impl<'a> WidgetNavBounds<LayerBrowserWidgetState> for LayerBrowserWidget<'a> {
    fn ensure_bounds(&self, state: &mut LayerBrowserWidgetState) {
        let max = self.layer_names.len();
        if let Some(selected) = state.state.selected() {
            if selected >= max {
                state.state.select(Some(max - 1));
            }
        }
    }
}

impl WidgetFocusTrait for LayerBrowserWidgetState {
    fn focus_on(&mut self, selected: bool) {
        self.is_toggled = selected;
        if self.is_toggled {
            self.title = "😍 Layer Browser".to_string();
        } else {
            self.title = "Layer Browser".to_string();
        }
    }

    fn is_focused(&self) -> bool {
        self.is_toggled
    }
}

impl<'a> StatefulWidget for LayerBrowserWidget<'a> {
    type State = LayerBrowserWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        // Create the List widget
        let list = List::new(self.layer_names.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(state.title.clone()),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        // Render the List widget with its state
        StatefulWidget::render(list, layout[0], buf, &mut state.state);

        // Create the Command widget
        let command: Paragraph =
            Paragraph::new(self.layer_commands[state.state.selected().unwrap_or(0)].clone())
                .block(Block::default().borders(Borders::ALL).title("Command"))
                .wrap(ratatui::widgets::Wrap { trim: true });

        Widget::render(command, layout[1], buf);
    }
}

