use ratatui::{
    layout::{Alignment, Rect},
    prelude::*,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct StatusBar {
    current_db: Option<String>,
    driver_name: Option<String>,
    server_version: Option<String>,
    username: Option<String>,
    status: String,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            current_db: None,
            driver_name: None,
            server_version: None,
            username: None,
            status: "READY".to_string(),
        }
    }

    pub fn set_current_db(&mut self, db: Option<String>) {
        self.current_db = db;
    }

    pub fn set_driver(&mut self, driver: &str) {
        self.driver_name = Some(driver.to_string());
    }

    pub fn set_server_version(&mut self, version: String) {
        self.server_version = Some(version);
    }

    pub fn set_username(&mut self, username: String) {
        self.username = Some(username);
    }


    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let db_info = self.current_db
            .as_ref()
            .map(|db| format!("DB: {}", db))
            .unwrap_or_else(|| "No DB".to_string());

        let driver = self.driver_name.as_deref().unwrap_or("sql");
        let version = self.server_version.as_deref().unwrap_or("Unknown");
        let version_info = format!("{}: {}", driver, version);

        let user_info = self.username
            .as_ref()
            .map(|u| format!("User: {}", u))
            .unwrap_or_else(|| "User: Unknown".to_string());

        let content = Line::from(vec![
            Span::styled("[SQLTUI] ", Style::default().fg(Color::Green).bold()),
            Span::styled(&self.status, Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(&user_info, Style::default().fg(Color::Magenta)),
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
