use ratatui::{
    layout::{Alignment, Rect},
    prelude::*,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct Input {
    input: String,
    mode: InputMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Command,
    SQL,
}

impl Input {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            mode: InputMode::Command,
        }
    }


    pub fn get_input(&self) -> &str {
        &self.input
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }

    pub fn get_mode(&self) -> &InputMode {
        &self.mode
    }

    pub fn add_char(&mut self, ch: char) {
        self.input.push(ch);
    }

    pub fn delete_char(&mut self) {
        self.input.pop();
    }

    pub fn clear(&mut self) {
        self.input.clear();
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let mode_text = match self.mode {
            InputMode::Command => "[CMD_MODE]",
            InputMode::SQL => "[SQL_MODE]",
        };

        let prompt = match self.mode {
            InputMode::Command => "mysql> ",
            InputMode::SQL => "sql> ",
        };

        let content = Line::from(vec![
            Span::styled(mode_text, Style::default().fg(Color::Yellow).bold()),
            Span::raw(" > "),
            Span::styled(prompt, Style::default().fg(Color::Green)),
            Span::styled(&self.input, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::White)), // 光标
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));

        let paragraph = Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, area);
    }
}
