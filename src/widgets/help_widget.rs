use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from("FreightView Help. Press 'ESC' to exit this help window."),
        Line::from("Use arrows to move around tree and list"),
        Line::from("Press 'TAB' to switch between tree and list"),
        Line::from("Press 'SPACEBAR' to open or close the tree item (directory)"),
        Line::from(
            "Press 'CTRL+F' to open search bar and filter the tree for the items that interest you",
        ),
        Line::from("Press 'q' to quit the app"),
        Line::from("Press 'h' to show this help"),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, area);
}
