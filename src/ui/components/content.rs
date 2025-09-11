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
    data_scroll_offset: usize,
    data_horizontal_scroll: usize,
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
            data_scroll_offset: 0,
            data_horizontal_scroll: 0,
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

    pub fn scroll_data_up(&mut self) {
        if self.data_scroll_offset > 0 {
            self.data_scroll_offset -= 1;
        }
    }

    pub fn scroll_data_down(&mut self) {
        self.data_scroll_offset += 1;
    }

    pub fn scroll_data_left(&mut self) {
        if self.data_horizontal_scroll > 0 {
            self.data_horizontal_scroll -= 1;
        }
    }

    pub fn scroll_data_right(&mut self) {
        self.data_horizontal_scroll += 1;
    }

    pub fn reset_data_scroll(&mut self) {
        self.data_scroll_offset = 0;
        self.data_horizontal_scroll = 0;
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

        // 如果内容不需要滚动，重置滚动位置
        if !self.can_scroll_schema(available_height) {
            self.schema_scroll_offset = 0;
        } else {
            // 限制滚动偏移量，确保不会滚动超出范围
            let max_scroll = total_rows.saturating_sub(max_rows);
            if self.schema_scroll_offset > max_scroll {
                self.schema_scroll_offset = max_scroll;
            }
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
        // 计算可显示的行数和列数
        let available_height = area.height as usize;
        let available_width = area.width as usize;
        let header_height = 1;
        let max_rows = available_height.saturating_sub(header_height + 2); // 减去边框高度
        
        // 限制垂直滚动
        let total_rows = self.table_rows.len();
        if self.data_scroll_offset >= total_rows {
            self.data_scroll_offset = total_rows.saturating_sub(1);
        }
        
        // 如果内容不需要垂直滚动，重置滚动位置
        if total_rows <= max_rows {
            self.data_scroll_offset = 0;
        }
        
        // 计算要显示的行范围
        let start_row = self.data_scroll_offset;
        let end_row = (start_row + max_rows).min(total_rows);
        
        // 计算要显示的列范围
        let total_cols = self.table_headers.len();
        let col_width = 15; // 每列固定宽度
        let max_cols = (available_width / col_width).max(1);
        
        // 限制水平滚动
        if self.data_horizontal_scroll >= total_cols {
            self.data_horizontal_scroll = total_cols.saturating_sub(1);
        }
        
        if total_cols <= max_cols {
            self.data_horizontal_scroll = 0;
        }
        
        let start_col = self.data_horizontal_scroll;
        let end_col = (start_col + max_cols).min(total_cols);
        
        // 创建要显示的行
        let rows: Vec<ratatui::widgets::Row> = self.table_rows
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx >= start_row && *idx < end_row)
            .map(|(_, row)| {
                let visible_cells: Vec<String> = row
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| *idx >= start_col && *idx < end_col)
                    .map(|(_, cell)| cell.clone())
                    .collect();
                ratatui::widgets::Row::new(visible_cells)
            })
            .collect();

        // 创建要显示的列头
        let visible_headers: Vec<String> = self.table_headers
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx >= start_col && *idx < end_col)
            .map(|(_, header)| header.clone())
            .collect();

        // 设置列宽
        let widths: Vec<Constraint> = (0..visible_headers.len())
            .map(|_| Constraint::Length(col_width as u16))
            .collect();

        // 创建标题，显示滚动信息
        let scroll_info = if total_rows > max_rows || total_cols > max_cols {
            format!(" (↑↓←→滚动) 行{}/{} 列{}/{}", 
                start_row + 1, total_rows, 
                start_col + 1, total_cols)
        } else {
            String::new()
        };
        
        let title = if let Some(table_name) = &self.current_table_name {
            format!("表数据 - {}{}", table_name, scroll_info)
        } else {
            format!("表数据{}", scroll_info)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));

        let inner_area = block.inner(area);

        let table = Table::new(rows, &widths)
            .header(
                ratatui::widgets::Row::new(visible_headers)
                .style(Style::default().fg(Color::Yellow).bold())
            )
            .block(Block::default().borders(Borders::NONE))
            .column_spacing(1);

        frame.render_widget(block, area);
        frame.render_widget(table, inner_area);
    }
}
