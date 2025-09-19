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
    show_suggestions: bool,
    suggestion_index: usize,
    // 光标位置（按字符计数，不是字节）
    cursor_pos: usize,
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
            show_suggestions: false,
            suggestion_index: 0,
            cursor_pos: 0,
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
        let byte_idx = self.byte_index_for_char_pos(self.cursor_pos);
        self.input.insert(byte_idx, ch);
        self.cursor_pos += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos == 0 { return; }
        let prev_char_pos = self.cursor_pos - 1;
        let start = self.byte_index_for_char_pos(prev_char_pos);
        let end = self.byte_index_for_char_pos(self.cursor_pos);
        self.input.replace_range(start..end, "");
        self.cursor_pos = prev_char_pos;
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
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

    pub fn toggle_suggestions(&mut self) {
        self.show_suggestions = !self.show_suggestions;
        self.suggestion_index = 0;
    }

    pub fn hide_suggestions(&mut self) {
        self.show_suggestions = false;
        self.suggestion_index = 0;
    }

    pub fn get_current_suggestion(&self) -> Option<String> {
        if self.show_suggestions {
            let suggestions = self.get_autocomplete_suggestions(&self.input);
            suggestions.get(self.suggestion_index).cloned()
        } else {
            None
        }
    }

    pub fn next_suggestion(&mut self) {
        if self.show_suggestions {
            let suggestions = self.get_autocomplete_suggestions(&self.input);
            if !suggestions.is_empty() {
                self.suggestion_index = (self.suggestion_index + 1) % suggestions.len();
            }
        }
    }

    pub fn prev_suggestion(&mut self) {
        if self.show_suggestions {
            let suggestions = self.get_autocomplete_suggestions(&self.input);
            if !suggestions.is_empty() {
                self.suggestion_index = if self.suggestion_index == 0 {
                    suggestions.len() - 1
                } else {
                    self.suggestion_index - 1
                };
            }
        }
    }

    pub fn is_showing_suggestions(&self) -> bool {
        self.show_suggestions
    }

    pub fn get_autocomplete_suggestions(&self, input: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let input_trimmed = input.trim();
        
        // 如果输入为空，返回常用关键字
        if input_trimmed.is_empty() {
            return vec![
                "SELECT".to_string(),
                "SHOW".to_string(),
                "USE".to_string(),
                "DESCRIBE".to_string(),
                "EXPLAIN".to_string(),
            ];
        }
        
        let input_lower = input_trimmed.to_lowercase();
        
        // SQL关键字自动补全
        let sql_keywords = vec![
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP",
            "ALTER", "USE", "SHOW", "DESCRIBE", "EXPLAIN", "JOIN", "LEFT", "RIGHT", "INNER",
            "OUTER", "ON", "GROUP", "BY", "ORDER", "HAVING", "LIMIT", "OFFSET", "DISTINCT",
            "COUNT", "SUM", "AVG", "MIN", "MAX", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN",
            "IS", "NULL", "TRUE", "FALSE", "ASC", "DESC", "AS", "UNION", "ALL", "EXISTS",
            "DATABASES", "TABLES", "COLUMNS", "INDEX", "INDEXES", "PROCESSLIST", "STATUS",
            "VARIABLES", "GRANTS", "PRIVILEGES", "USERS", "FUNCTIONS", "PROCEDURES", "TRIGGERS"
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

        // 语法高亮的输入内容（带光标，反色覆盖当前字符）
        self.cursor_pos = self.cursor_pos.min(self.input.chars().count());
        let byte_idx = self.byte_index_for_char_pos(self.cursor_pos);
        let (before, after) = self.input.split_at(byte_idx);
        let styled_before = self.highlight_sql_syntax(before);

        let mut content_spans = vec![
            Span::styled(mode_text, Style::default().fg(Color::Yellow).bold()),
            Span::raw(" > "),
            Span::styled(&prompt, Style::default().fg(Color::Green)),
        ];
        
        content_spans.extend(styled_before);
        // 光标覆盖字符：若有字符则反色显示该字符，否则反色空格
        let mut after_chars = after.chars();
        if let Some(cursor_ch) = after_chars.next() {
            content_spans.push(Span::styled(
                cursor_ch.to_string(),
                Style::default().add_modifier(Modifier::REVERSED),
            ));
            let rest: String = after_chars.collect();
            let styled_rest = self.highlight_sql_syntax(&rest);
            content_spans.extend(styled_rest);
        } else {
            content_spans.push(Span::styled(
                " ",
                Style::default().add_modifier(Modifier::REVERSED),
            ));
        }

        let content = Line::from(content_spans);

        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));

        let paragraph = Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, area);

        // 在SQL模式下显示建议
        if self.mode == InputMode::SQL && self.show_suggestions {
            self.render_suggestions(frame, area);
        }
    }

    // 光标移动与边界
    pub fn move_cursor_start(&mut self) { self.cursor_pos = 0; }
    pub fn move_cursor_end(&mut self) { self.cursor_pos = self.input.chars().count(); }
    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 { self.cursor_pos -= 1; }
    }
    pub fn move_cursor_right(&mut self) {
        let len = self.input.chars().count();
        if self.cursor_pos < len { self.cursor_pos += 1; }
    }

    pub fn move_word_left(&mut self) {
        if self.cursor_pos == 0 { return; }
        let chars: Vec<char> = self.input.chars().collect();
        let mut i = self.cursor_pos;
        // 跳过空白
        while i > 0 && chars[i-1].is_whitespace() { i -= 1; }
        // 跳过单词字符
        while i > 0 && is_word_char(chars[i-1]) { i -= 1; }
        self.cursor_pos = i;
    }

    pub fn move_word_right(&mut self) {
        let chars: Vec<char> = self.input.chars().collect();
        let mut i = self.cursor_pos;
        let n = chars.len();
        // 跳过单词字符
        while i < n && is_word_char(chars[i]) { i += 1; }
        // 跳过空白
        while i < n && chars[i].is_whitespace() { i += 1; }
        self.cursor_pos = i;
    }

    fn byte_index_for_char_pos(&self, char_pos: usize) -> usize {
        if char_pos == 0 { return 0; }
        let mut count = 0;
        for (byte_idx, _ch) in self.input.char_indices() {
            if count == char_pos { return byte_idx; }
            count += 1;
        }
        self.input.len()
    }

    fn highlight_sql_syntax(&self, input: &str) -> Vec<Span<'static>> {
        if self.mode != InputMode::SQL {
            return vec![Span::styled(input.to_string(), Style::default().fg(Color::White))];
        }

        // 如果输入为空，直接返回
        if input.is_empty() {
            return vec![];
        }

        let mut spans = Vec::new();
        let mut chars = input.chars().peekable();
        let mut current_word = String::new();
        
        while let Some(ch) = chars.next() {
            if ch.is_whitespace() {
                // 如果当前有单词，先处理单词
                if !current_word.is_empty() {
                    let style = self.get_word_style(&current_word);
                    spans.push(Span::styled(current_word.clone(), style));
                    current_word.clear();
                }
                // 添加空格
                spans.push(Span::raw(" "));
            } else {
                current_word.push(ch);
            }
        }
        
        // 处理最后一个单词
        if !current_word.is_empty() {
            let style = self.get_word_style(&current_word);
            spans.push(Span::styled(current_word, style));
        }
        
        spans
    }

    fn get_word_style(&self, word: &str) -> Style {
        let word_upper = word.to_uppercase();
        match word_upper.as_str() {
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
        }
    }

    fn render_suggestions(&self, frame: &mut Frame, input_area: Rect) {
        let suggestions = self.get_autocomplete_suggestions(&self.input);
        if suggestions.is_empty() {
            return;
        }

        // 在输入框下方显示建议
        let suggestion_area = Rect {
            x: input_area.x,
            y: input_area.y + input_area.height,
            width: input_area.width,
            height: std::cmp::min(suggestions.len() as u16 + 2, 8), // 最多显示8行
        };

        let mut suggestion_lines = Vec::new();
        for (i, suggestion) in suggestions.iter().enumerate() {
            let style = if i == self.suggestion_index {
                Style::default().fg(Color::Yellow).bg(Color::Blue)
            } else {
                Style::default().fg(Color::Cyan)
            };
            suggestion_lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(suggestion, style),
            ]));
        }

        let suggestion_block = Block::default()
            .borders(Borders::ALL)
            .title("建议")
            .style(Style::default().fg(Color::Blue));

        let suggestion_paragraph = Paragraph::new(suggestion_lines)
            .block(suggestion_block)
            .alignment(Alignment::Left);

        frame.render_widget(suggestion_paragraph, suggestion_area);
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' || ch == '.'
}
