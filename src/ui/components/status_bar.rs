use ratatui::{
    layout::{Alignment, Rect},
    prelude::*,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct StatusBar {
    current_db: Option<String>,
    mysql_version: Option<String>,
    status: String,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            current_db: None,
            mysql_version: None,
            status: "READY".to_string(),
        }
    }

    pub fn set_current_db(&mut self, db: Option<String>) {
        self.current_db = db;
    }

    pub fn set_mysql_version(&mut self, version: String) {
        self.mysql_version = Some(version);
    }


    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let db_info = self.current_db
            .as_ref()
            .map(|db| format!("DB: {}", db))
            .unwrap_or_else(|| "No DB".to_string());

        let version_info = self.mysql_version
            .as_ref()
            .map(|v| format!("MySQL: {}", v))
            .unwrap_or_else(|| "MySQL: Unknown".to_string());

        let content = Line::from(vec![
            Span::styled("[MYSQL_CLIENT] ", Style::default().fg(Color::Green).bold()),
            Span::styled(&self.status, Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(&db_info, Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::styled(&version_info, Style::default().fg(Color::Blue)),
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
