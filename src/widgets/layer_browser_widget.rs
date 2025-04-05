use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListState, Paragraph, StatefulWidget, Widget};
use crate::widgets::navigation_traits::Navigation;

pub struct LayerBrowserWidget<'a> {
    layer_names: &'a Vec<String>,
    layer_commands: &'a Vec<String>,
}

impl<'a> LayerBrowserWidget<'a> {
    pub fn new(layer_names: &'a Vec<String>, layer_commands: &'a Vec<String> ) -> Self {
        Self { layer_names, 
               layer_commands }
    }
}

impl Navigation<ListState> for ListState {
    fn next(state: &mut ListState, max: usize) {
        if let Some(selected) = state.selected() {
            if selected < max - 1 {
                state.select(Some(selected + 1));
            }
        } else {
            state.select(Some(0));
        }        
    }

    fn prev(state: &mut ListState) {
        if let Some(selected) = state.selected() {
            if selected > 0 {
                state.select(Some(selected - 1));
            }
        } else {
            state.select(Some(0));
        }
    }
}

impl<'a> StatefulWidget for LayerBrowserWidget<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State){ 
        let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(area);
        
        // Create the List widget
        let list = List::new(self.layer_names.clone())
        .block(Block::default().borders(Borders::ALL).title("Layers"))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        // Render the List widget with its state
        StatefulWidget::render(list, layout[0], buf, state);



        // Create the Command widget
        let command :Paragraph = Paragraph::new(self.layer_commands[state.selected().unwrap_or(0)].clone())
        .block(Block::default().borders(Borders::ALL).title("Command"))
        .wrap(ratatui::widgets::Wrap { trim: true });
        
        Widget::render(command, layout[1], buf);
    }
}