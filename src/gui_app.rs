use crate::docker_image_utils::ImageRepr;
use crate::widgets::focus_traits::WidgetFocusTrait;
use crate::widgets::multitree_browser_widget::{
    MultiTreeBrowserWidget, MultiTreeBrowserWidgetState,
};
use crate::widgets::navigation_traits::{WidgetNav, WidgetNavBounds};
use crate::widgets::searchbar::{SearchBarWidget, SearchBarWidgetState};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    widgets::ListState,
};
use std::io;
use std::time::Duration;

use crate::widgets::layer_browser_widget::{LayerBrowserWidget, LayerBrowserWidgetState};

enum Focus {
    List,
    Tree,
    SearchBar,
}

pub struct App {
    item: ImageRepr,
    exit: bool,
    tree_state: MultiTreeBrowserWidgetState,
    list_state: LayerBrowserWidgetState,
    layer_names: Vec<String>,
    layer_commands: Vec<String>,
    focus: Focus,
    search_bar_state: SearchBarWidgetState,
}

impl App {
    pub fn new(item: ImageRepr) -> App {
        let layer_names: Vec<String> = App::layer_names_from_img(&item);
        let layer_commands: Vec<String> = item
            .layers
            .iter()
            .map(|layer| layer.command.clone())
            .collect();

        if layer_names.len() == 0 {
            panic!("No layers found in image");
        }
        if layer_names.len() != layer_commands.len() {
            panic!("Layer names and commands are not the same length");
        }

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut app = App {
            item,
            exit: false,
            layer_names: layer_names.clone(),
            layer_commands,
            list_state: LayerBrowserWidgetState::new(),
            focus: Focus::List,
            tree_state: MultiTreeBrowserWidgetState::new("", &layer_names[..]),
            search_bar_state: SearchBarWidgetState::new(),
        };

        app.adjust_tree_state_to_list();
        return app;
    }

    fn layer_names_from_img(img: &ImageRepr) -> Vec<String> {
        img.layers.iter().map(|layer| layer.name.clone()).collect()
    }

    fn deselect_all(&mut self) {
        self.list_state.focus_on(false);
        self.tree_state.focus_on(false);
    }

    fn circle_focus(&mut self) {
        match self.focus {
            Focus::List => {
                self.deselect_all();
                self.tree_state.focus_on(true);
                self.focus = Focus::Tree;
            }
            Focus::Tree => {
                self.deselect_all();
                self.list_state.focus_on(true);
                self.focus = Focus::List;
            }
            Focus::SearchBar => {}
        }
    }

    fn adjust_tree_state_to_list(&mut self) {
        let selected = self.list_state.selected().unwrap_or(0);
        if selected >= self.layer_names.len() {
            return;
        }
        let selected_layer = &self.layer_names[selected];
        self.tree_state.current_layer = selected_layer.to_string();
    }

    fn adjust_tree_state_to_search_bar_content(&mut self) {
        let search_string = self.search_bar_state.get();
        self.tree_state.set_search_string(&search_string);
    }

    fn next(&mut self) {
        match self.focus {
            Focus::List => {
                self.list_state.next();
                self.adjust_tree_state_to_list();
            }
            Focus::Tree => {
                self.tree_state.next();
            }
            Focus::SearchBar => {}
        }
    }

    fn previous(&mut self) {
        match self.focus {
            Focus::List => {
                self.list_state.prev();
                self.adjust_tree_state_to_list();
            }
            Focus::Tree => {
                self.tree_state.prev();
            }
            Focus::SearchBar => {}
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let vlayout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(100), Constraint::Length(3)].as_ref())
            .split(area);

        let hlayout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(vlayout[0]);

        let layers_and_commands = LayerBrowserWidget::new(&self.layer_names, &self.layer_commands);

        layers_and_commands.ensure_bounds(&mut self.list_state);
        frame.render_stateful_widget(layers_and_commands, hlayout[0], &mut self.list_state);

        let tree_widget = MultiTreeBrowserWidget::new(&self.item.layers[..]);
        frame.render_stateful_widget(tree_widget, hlayout[1], &mut self.tree_state);

        let search = SearchBarWidget::new();
        frame.render_stateful_widget(search, vlayout[1], &mut self.search_bar_state);
    }

    fn get_all_key_events() -> Vec<KeyEvent> {
        let mut key_events = Vec::new();
        loop {
            let event = event::poll(Duration::from_millis(0));
            if event.is_err() {
                break;
            }
            let event = event.unwrap();
            if !event {
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
                Focus::SearchBar => match key_event.code {
                    KeyCode::Char(c) => {
                        self.search_bar_state.push_c(c);
                        self.adjust_tree_state_to_search_bar_content();
                    }
                    KeyCode::Backspace => {
                        self.search_bar_state.pop_c();
                        self.adjust_tree_state_to_search_bar_content();
                    }
                    KeyCode::Enter => {
                        self.focus = Focus::Tree;
                    }
                    KeyCode::Esc => {
                        self.search_bar_state.focus_on(false);
                        self.list_state.focus_on(true);
                        self.focus = Focus::List;
                    }
                    _ => {}
                },
                _ => {
                    match key_event.code {
                        KeyCode::Down => self.next(),                   // Move selection down
                        KeyCode::Up => self.previous(),                 // Move selection up
                        KeyCode::Tab => self.circle_focus(), // Switch between list and tree
                        KeyCode::Char(' ') => self.tree_state.expand(), // Expand tree
                        KeyCode::Char('q') => self.exit = true, // Quit
                        KeyCode::Char('f')
                            if key_event
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            self.deselect_all();
                            self.search_bar_state.focus_on(true);
                            self.focus = Focus::SearchBar
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
}
