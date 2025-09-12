use ratatui::{
    layout::{Alignment, Rect},
    prelude::*,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct Input {
    input: String,
    mode: InputMode,
    history: Vec<String>,
    history_index: usize,
    current_db: Option<String>,
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
            history: Vec::new(),
            history_index: 0,
            current_db: None,
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

    pub fn set_current_db(&mut self, db_name: Option<String>) {
        self.current_db = db_name;
    }

    pub fn add_to_history(&mut self, command: String) {
        if !command.trim().is_empty() && self.history.last() != Some(&command) {
            self.history.push(command);
            self.history_index = self.history.len();
        }
    }

    pub fn get_history_up(&mut self) -> Option<String> {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.history.get(self.history_index).cloned()
        } else {
            None
        }
    }

    pub fn get_history_down(&mut self) -> Option<String> {
        if self.history_index < self.history.len() {
            self.history_index += 1;
            self.history.get(self.history_index).cloned()
        } else {
            None
        }
    }

    pub fn reset_history_index(&mut self) {
        self.history_index = self.history.len();
    }

    pub fn get_autocomplete_suggestions(&self, input: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let input_lower = input.to_lowercase();
        
        // SQL关键字自动补全
        let sql_keywords = vec![
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP",
            "ALTER", "USE", "SHOW", "DESCRIBE", "EXPLAIN", "JOIN", "LEFT", "RIGHT", "INNER",
            "OUTER", "ON", "GROUP", "BY", "ORDER", "HAVING", "LIMIT", "OFFSET", "DISTINCT",
            "COUNT", "SUM", "AVG", "MIN", "MAX", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN",
            "IS", "NULL", "TRUE", "FALSE", "ASC", "DESC", "AS", "UNION", "ALL", "EXISTS"
        ];
        
        for keyword in sql_keywords {
            if keyword.to_lowercase().starts_with(&input_lower) {
                suggestions.push(keyword.to_string());
            }
        }
        
        // 限制建议数量
        suggestions.truncate(10);
        suggestions
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let mode_text = match self.mode {
            InputMode::Command => "[CMD_MODE]",
            InputMode::SQL => "[SQL_MODE]",
        };

        // mycli风格的提示符
        let prompt = match self.mode {
            InputMode::Command => "mysql> ".to_string(),
            InputMode::SQL => {
                if let Some(db) = &self.current_db {
                    format!("{}@localhost:{}> ", "root", db)
                } else {
                    "root@localhost:(none)> ".to_string()
                }
            },
        };

        // 语法高亮的输入内容
        let styled_input = self.highlight_sql_syntax(&self.input);

        let mut content_spans = vec![
            Span::styled(mode_text, Style::default().fg(Color::Yellow).bold()),
            Span::raw(" > "),
            Span::styled(&prompt, Style::default().fg(Color::Green)),
        ];
        
        content_spans.extend(styled_input);
        content_spans.push(Span::styled("█", Style::default().fg(Color::White))); // 光标

        let content = Line::from(content_spans);

        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));

        let paragraph = Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, area);
    }

    fn highlight_sql_syntax(&self, input: &str) -> Vec<Span<'static>> {
        if self.mode != InputMode::SQL {
            return vec![Span::styled(input.to_string(), Style::default().fg(Color::White))];
        }

        let mut spans = Vec::new();
        let words: Vec<&str> = input.split_whitespace().collect();
        
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            
            let word_upper = word.to_uppercase();
            let style = match word_upper.as_str() {
                "SELECT" | "FROM" | "WHERE" | "INSERT" | "UPDATE" | "DELETE" | "CREATE" | "DROP" |
                "ALTER" | "USE" | "SHOW" | "DESCRIBE" | "EXPLAIN" | "JOIN" | "LEFT" | "RIGHT" |
                "INNER" | "OUTER" | "ON" | "GROUP" | "BY" | "ORDER" | "HAVING" | "LIMIT" |
                "OFFSET" | "DISTINCT" | "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "AND" | "OR" |
                "NOT" | "IN" | "LIKE" | "BETWEEN" | "IS" | "NULL" | "TRUE" | "FALSE" | "ASC" |
                "DESC" | "AS" | "UNION" | "ALL" | "EXISTS" => {
                    Style::default().fg(Color::Cyan).bold()
                },
                _ if word.starts_with('\'') && word.ends_with('\'') => {
                    Style::default().fg(Color::Green) // 字符串
                },
                _ if word.starts_with('"') && word.ends_with('"') => {
                    Style::default().fg(Color::Green) // 字符串
                },
                _ if word.parse::<i64>().is_ok() || word.parse::<f64>().is_ok() => {
                    Style::default().fg(Color::Yellow) // 数字
                },
                _ => Style::default().fg(Color::White), // 普通文本
            };
            
            spans.push(Span::styled((*word).to_string(), style));
        }
        
        spans
    }
}
