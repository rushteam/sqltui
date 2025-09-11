use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, Paragraph, Table},
    Frame,
};
use crate::models::SchemaColumn;

pub enum ContentType {
    Welcome,
    Database,
    Tables,
    TableSchema,
    TableData,
    Help,
    Error,
}

pub struct Content {
    content_type: ContentType,
    content: String,
    table_headers: Vec<String>,
    table_rows: Vec<Vec<String>>,
    schema_columns: Vec<SchemaColumn>,
    table_comment: Option<String>,
    current_table_name: Option<String>,
    schema_scroll_offset: usize,
}

impl Content {
    pub fn new() -> Self {
        Self {
            content_type: ContentType::Welcome,
            content: String::new(),
            table_headers: Vec::new(),
            table_rows: Vec::new(),
            schema_columns: Vec::new(),
            table_comment: None,
            current_table_name: None,
            schema_scroll_offset: 0,
        }
    }

    pub fn set_content_type(&mut self, content_type: ContentType) {
        self.content_type = content_type;
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    pub fn set_table_schema(&mut self, columns: Vec<SchemaColumn>, comment: Option<String>) {
        self.schema_columns = columns;
        self.table_comment = comment;
        self.content_type = ContentType::TableSchema;
    }

    pub fn set_table_name(&mut self, table_name: String) {
        self.current_table_name = Some(table_name);
    }

    pub fn set_table_data(&mut self, headers: Vec<String>, rows: Vec<Vec<String>>) {
        self.table_headers = headers;
        self.table_rows = rows;
        self.content_type = ContentType::TableData;
    }

    pub fn get_content_type(&self) -> &ContentType {
        &self.content_type
    }

    pub fn scroll_schema_up(&mut self) {
        if self.schema_scroll_offset > 0 {
            self.schema_scroll_offset -= 1;
        }
    }

    pub fn scroll_schema_down(&mut self) {
        self.schema_scroll_offset += 1;
    }

    pub fn can_scroll_schema(&self, available_height: usize) -> bool {
        let total_rows = self.schema_columns.len();
        let max_rows = available_height.saturating_sub(3); // 减去边框和表头高度
        total_rows > max_rows
    }

    pub fn reset_schema_scroll(&mut self) {
        self.schema_scroll_offset = 0;
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));

        match self.content_type {
            ContentType::Welcome => {
                let paragraph = Paragraph::new(self.content.clone())
                    .block(block)
                    .wrap(ratatui::widgets::Wrap { trim: true });
                frame.render_widget(paragraph, area);
            }
            ContentType::TableSchema => {
                self.render_table_schema(frame, area);
            }
            ContentType::TableData => {
                self.render_table_data(frame, area);
            }
            _ => {
                let paragraph = Paragraph::new(self.content.clone())
                    .block(block)
                    .wrap(ratatui::widgets::Wrap { trim: true });
                frame.render_widget(paragraph, area);
            }
        }
    }

    fn render_table_schema(&mut self, frame: &mut Frame, area: Rect) {
        // 计算可显示的行数
        let available_height = area.height as usize;
        let total_rows = self.schema_columns.len();
        let max_rows = available_height.saturating_sub(3); // 减去边框和表头高度
        
        // 根据是否需要滚动来显示不同的标题
        let scroll_hint = if total_rows > max_rows {
            " (↑↓滚动)"
        } else {
            ""
        };
        
        let title = if let Some(v) = &self.current_table_name {
            if !v.is_empty() {
                format!("表结构 - {}{}", v, scroll_hint)
            } else {
                format!("表结构{}", scroll_hint)
            }
        } else {
            format!("表结构{}", scroll_hint)
        };

        // 创建主框
        let main_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));

        // 在框内创建布局
        let inner_area = main_block.inner(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // 表格区域
            ])
            .split(inner_area);

        // 渲染主框
        frame.render_widget(main_block, area);

        // 计算可显示的行数
        let available_height = chunks[0].height as usize;
        let header_height = 1; // 表头占1行
        let max_rows = available_height.saturating_sub(header_height);

        // 限制滚动偏移量
        if self.schema_scroll_offset >= total_rows {
            self.schema_scroll_offset = total_rows.saturating_sub(1);
        }
        
        // 如果内容不需要滚动，重置滚动位置
        if !self.can_scroll_schema(available_height) {
            self.schema_scroll_offset = 0;
        }

        // 计算要显示的行范围
        let start_idx = self.schema_scroll_offset;
        let end_idx = (start_idx + max_rows).min(total_rows);

        // 创建要显示的行
        let rows: Vec<ratatui::widgets::Row> = self.schema_columns
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx >= start_idx && *idx < end_idx)
            .map(|(_, col)| {
                let nullable = if col.is_nullable { "YES" } else { "NO" };
                let default = col.default_value.as_deref().unwrap_or("");
                let extra = col.extra.as_deref().unwrap_or("");
                let comment = col.comment.as_deref().unwrap_or("");

                ratatui::widgets::Row::new(vec![
                    col.name.clone(),
                    col.data_type.clone(),
                    nullable.to_string(),
                    default.to_string(),
                    extra.to_string(),
                    comment.to_string(),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(20),
            Constraint::Length(15),
            Constraint::Length(8),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Min(20),
        ];

        let table = Table::new(rows, widths)
            .header(
                ratatui::widgets::Row::new(vec![
                    "字段名", "类型", "可空", "默认值", "额外", "注释"
                ])
                .style(Style::default().fg(Color::Yellow).bold())
            )
            .block(Block::default().borders(Borders::NONE))
            .column_spacing(1);

        frame.render_widget(table, chunks[0]);
    }

    fn render_table_data(&mut self, frame: &mut Frame, area: Rect) {
        let rows: Vec<ratatui::widgets::Row> = self.table_rows
            .iter()
            .map(|row| {
                ratatui::widgets::Row::new(row.clone())
            })
            .collect();

        let widths: Vec<Constraint> = self.table_headers
            .iter()
            .map(|_| Constraint::Min(10))
            .collect();

        let table = Table::new(rows, &widths)
            .header(
                ratatui::widgets::Row::new(self.table_headers.clone())
                .style(Style::default().fg(Color::Yellow).bold())
            )
            .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Green)))
            .column_spacing(1);

        frame.render_widget(table, area);
    }
}
