use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use crate::models::{Database, Table};

pub struct Sidebar {
    databases: Vec<Database>,
    tables: Vec<Table>,
    show_databases: bool,
    current_db: Option<String>,
    db_list_state: ListState,
    table_list_state: ListState,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            databases: Vec::new(),
            tables: Vec::new(),
            show_databases: true,
            current_db: None,
            db_list_state: ListState::default(),
            table_list_state: ListState::default(),
        }
    }

    pub fn set_databases(&mut self, databases: Vec<Database>) {
        self.databases = databases;
        self.db_list_state.select(Some(0));
    }

    pub fn set_tables(&mut self, tables: Vec<Table>) {
        self.tables = tables;
        self.table_list_state.select(Some(0));
    }

    pub fn set_show_databases(&mut self, show: bool) {
        self.show_databases = show;
    }

    pub fn set_current_db(&mut self, db: Option<String>) {
        self.current_db = db;
    }

    pub fn next_item(&mut self) {
        if self.show_databases {
            let i = match self.db_list_state.selected() {
                Some(i) => {
                    if i >= self.databases.len().saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.db_list_state.select(Some(i));
        } else {
            let i = match self.table_list_state.selected() {
                Some(i) => {
                    if i >= self.tables.len().saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.table_list_state.select(Some(i));
        }
    }

    pub fn previous_item(&mut self) {
        if self.show_databases {
            let i = match self.db_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.databases.len().saturating_sub(1)
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.db_list_state.select(Some(i));
        } else {
            let i = match self.table_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.tables.len().saturating_sub(1)
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.table_list_state.select(Some(i));
        }
    }

    pub fn get_selected_database(&self) -> Option<&Database> {
        if self.show_databases {
            self.db_list_state.selected().and_then(|i| self.databases.get(i))
        } else {
            None
        }
    }

    pub fn get_selected_table(&self) -> Option<&Table> {
        if !self.show_databases {
            self.table_list_state.selected().and_then(|i| self.tables.get(i))
        } else {
            None
        }
    }

    pub fn get_show_databases(&self) -> bool {
        self.show_databases
    }

    pub fn get_tables_count(&self) -> usize {
        self.tables.len()
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // 标题
        let title = if self.show_databases {
            "数据库列表"
        } else {
            &format!("表列表 - {}", self.current_db.as_deref().unwrap_or(""))
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
                Constraint::Min(0),    // 列表区域
                Constraint::Length(1), // 状态信息
                Constraint::Length(1), // 帮助信息
            ])
            .split(inner_area);

        // 渲染主框
        frame.render_widget(main_block, area);

        // 列表
        if self.show_databases {
            let items: Vec<ListItem> = self.databases
                .iter()
                .map(|db| {
                    let _comment = db.charset.as_deref().unwrap_or("");
                    let table_count = db.table_count.map(|c| format!(" ({} 表)", c)).unwrap_or_default();
                    ListItem::new(Line::from(vec![
                        Span::styled(&db.name, Style::default().fg(Color::White)),
                        Span::styled(table_count, Style::default().fg(Color::Gray)),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::NONE))
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Green).bold());
            
            frame.render_stateful_widget(list, chunks[0], &mut self.db_list_state);
        } else {
            let items: Vec<ListItem> = self.tables
                .iter()
                .map(|table| {
                    let comment = table.comment.as_deref().unwrap_or("");
                    ListItem::new(Line::from(vec![
                        Span::styled(&table.name, Style::default().fg(Color::White)),
                        if !comment.is_empty() {
                            Span::styled(format!(" - {}", comment), Style::default().fg(Color::Gray))
                        } else {
                            Span::raw("")
                        },
                    ]))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::NONE))
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Green).bold());
            
            frame.render_stateful_widget(list, chunks[0], &mut self.table_list_state);
        }

        // 状态信息（在框内底部）
        let status = if let Some(selected) = self.get_selected_database() {
            format!("选中: {}", selected.name)
        } else if let Some(selected) = self.get_selected_table() {
            format!("选中: {}", selected.name)
        } else {
            String::new()
        };

        let status_style = Style::default().fg(Color::Green);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(status).style(status_style),
            chunks[1]
        );

        // 帮助信息（在框内底部）
        let help_text = if self.show_databases {
            "Up/Down 移动 | Enter 选择 | d 详情"
        } else {
            "Up/Down 移动 | Enter 选择 | t 详情 | s 返回"
        };

        let help_style = Style::default().fg(Color::Gray);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(help_text).style(help_style),
            chunks[2]
        );
    }
}
