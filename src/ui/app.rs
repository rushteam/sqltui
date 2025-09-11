use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::io::Write;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    prelude::*,
    Terminal,
};
use std::io;

use crate::{
    config::Config,
    db::{DatabaseConnection, DatabaseQueries},
    ui::components::{Content, Input, Sidebar, StatusBar},
};

use crate::ui::components::content::ContentType;
use crate::ui::components::input::InputMode;

pub struct App {
    // 数据库相关
    db_queries: DatabaseQueries,
    
    // UI 组件
    sidebar: Sidebar,
    content: Content,
    status_bar: StatusBar,
    input: Input,
    
    // 状态
    current_db: Option<String>,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let dsn = config.get_dsn();
        let db_connection = DatabaseConnection::new(&dsn).await?;
        let pool = db_connection.get_pool().clone();
        let db_queries = DatabaseQueries::new(pool);

        let mut app = Self {
            db_queries,
            sidebar: Sidebar::new(),
            content: Content::new(),
            status_bar: StatusBar::new(),
            input: Input::new(),
            current_db: None,
        };

        // 初始化数据
        app.load_databases().await?;
        app.load_mysql_version().await?;

        Ok(app)
    }

    pub async fn run(&mut self) -> Result<()> {
        // 设置信号处理
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        })?;

        // 设置终端
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // 主循环
        let result = self.run_app(&mut terminal, running).await;

        // 无论成功还是失败，都要恢复终端状态
        self.cleanup_terminal(&mut terminal)?;

        result
    }
    
    fn cleanup_terminal<B: Backend + io::Write>(&self, terminal: &mut Terminal<B>) -> Result<()> {
        // 使用更安全的清理方式，忽略所有错误
        self.safe_cleanup_terminal(terminal);
        Ok(())
    }
    
    fn safe_cleanup_terminal<B: Backend + io::Write>(&self, terminal: &mut Terminal<B>) {
        // 强制刷新输出
        let _ = ratatui::backend::Backend::flush(terminal.backend_mut());
        
        // 确保显示光标
        let _ = terminal.show_cursor();
        
        // 退出备用屏幕
        let _ = execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen
        );
        
        // 恢复终端模式
        let _ = disable_raw_mode();
        
        // 再次强制刷新确保所有输出都被处理
        let _ = ratatui::backend::Backend::flush(terminal.backend_mut());
        
        // 额外清理：直接操作 stdout
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen
        );
        let _ = io::stdout().flush();
    }

    async fn run_app<B: Backend + io::Write>(&mut self, terminal: &mut Terminal<B>, running: Arc<AtomicBool>) -> Result<()> {
        loop {
            // 检查是否收到退出信号
            if !running.load(Ordering::SeqCst) {
                break;
            }
            
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if self.handle_key_event(key).await? {
                    break;
                }
            }
        }
        
        // 在退出前清理终端
        self.cleanup_terminal(terminal)?;
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // 状态栏
                Constraint::Min(0),    // 主内容区
                Constraint::Length(3), // 输入栏
            ])
            .split(f.area());

        // 主内容区
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // 侧边栏
                Constraint::Percentage(70), // 内容区
            ])
            .split(chunks[1]);

        // 渲染组件
        self.status_bar.render(f, chunks[0]);
        self.sidebar.render(f, main_chunks[0]);
        self.content.render(f, main_chunks[1]);
        self.input.render(f, chunks[2]);
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') => {
                // 按 q 键直接退出
                return Ok(true);
            }
            KeyCode::Char('c') => {
                // 只有 Ctrl+C 才退出
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(true);
                }
                // 否则继续处理其他逻辑
            }
            KeyCode::Esc => {
                // Esc 键用于层级导航
                self.handle_escape().await?;
            }
            KeyCode::Up => {
                self.sidebar.previous_item();
            }
            KeyCode::Down => {
                self.sidebar.next_item();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.input.get_mode() == &InputMode::SQL {
                    if let Err(e) = self.handle_sql_command().await {
                        // SQL 执行失败时显示错误，但不退出程序
                        self.content.set_content_type(ContentType::Error);
                        self.content.set_content(format!("SQL 执行错误: {}", e));
                    }
                } else {
                    self.handle_enter().await?;
                }
            }
            KeyCode::Char('d') => {
                self.handle_database_detail().await?;
            }
            KeyCode::Char('t') => {
                self.handle_table_detail().await?;
            }
            KeyCode::Char('s') => {
                self.handle_switch_database().await?;
            }
            KeyCode::Char('\\') => {
                self.input.set_mode(InputMode::SQL);
            }
            KeyCode::Char(ch) => {
                if self.input.get_mode() == &InputMode::SQL {
                    self.input.add_char(ch);
                }
            }
            KeyCode::Backspace => {
                if self.input.get_mode() == &InputMode::SQL {
                    self.input.delete_char();
                }
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_escape(&mut self) -> Result<()> {
        match self.content.get_content_type() {
            ContentType::TableSchema | ContentType::TableData => {
                // 从表结构/数据返回表列表
                self.content.set_content_type(ContentType::Tables);
                self.content.set_content(format!(
                    "数据库 '{}' 中有 {} 个表，请选择一个表查看其结构\n\n[HINT] Enter 查看结构 | t 详情 | s 返回数据库列表",
                    self.current_db.as_deref().unwrap_or(""),
                    self.sidebar.get_tables_count()
                ));
            }
            ContentType::Tables => {
                // 从表列表返回数据库列表
                self.sidebar.set_show_databases(true);
                self.current_db = None;
                self.status_bar.set_current_db(None);
                self.content.set_content_type(ContentType::Welcome);
                self.content.set_content("MYSQL CLIENT v1.0 - READY\n\n[INSTRUCTIONS]\n- Use Up/Down keys to navigate\n- Press Enter to select database/table\n- Type commands or SQL in bottom input\n- Press 'q' to exit\n\n[STATUS] CONNECTED".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_enter(&mut self) -> Result<()> {
        if self.sidebar.get_show_databases() {
            if let Some(db) = self.sidebar.get_selected_database() {
                let db_name = db.name.clone();
                self.current_db = Some(db_name.clone());
                self.status_bar.set_current_db(Some(db_name.clone()));
                self.sidebar.set_show_databases(false);
                self.sidebar.set_current_db(Some(db_name.clone()));
                self.content.set_content_type(ContentType::Database);
                self.content.set_content(format!("正在加载数据库 '{}' 的表...", db_name));
                if let Err(e) = self.load_tables().await {
                    self.content.set_content_type(ContentType::Error);
                    self.content.set_content(format!("加载表列表失败: {}", e));
                }
            }
        } else {
            if let Some(table) = self.sidebar.get_selected_table() {
                let table_name = table.name.clone();
                self.content.set_content_type(ContentType::TableSchema);
                self.content.set_content("正在加载表结构...".to_string());
                if let Err(e) = self.load_table_schema(table_name).await {
                    self.content.set_content_type(ContentType::Error);
                    self.content.set_content(format!("加载表结构失败: {}", e));
                }
            }
        }
        Ok(())
    }

    async fn handle_database_detail(&mut self) -> Result<()> {
        if let Some(db) = self.sidebar.get_selected_database() {
            let detail = format!(
                "数据库详情:\n名称: {}\n字符集: {}\n排序规则: {}\n表数量: {}",
                db.name,
                db.charset.as_deref().unwrap_or("未知"),
                db.collation.as_deref().unwrap_or("未知"),
                db.table_count.unwrap_or(0)
            );
            self.content.set_content_type(ContentType::Database);
            self.content.set_content(detail);
        }
        Ok(())
    }

    async fn handle_table_detail(&mut self) -> Result<()> {
        if let Some(table) = self.sidebar.get_selected_table() {
            let detail = format!(
                "表详情:\n名称: {}\n注释: {}\n行数: {}\n大小: {} MB\n引擎: {}",
                table.name,
                table.comment.as_deref().unwrap_or("无"),
                table.rows.unwrap_or(0),
                table.size.unwrap_or(0),
                table.engine.as_deref().unwrap_or("未知")
            );
            self.content.set_content_type(ContentType::Tables);
            self.content.set_content(detail);
        }
        Ok(())
    }

    async fn handle_switch_database(&mut self) -> Result<()> {
        if self.current_db.is_some() {
            self.sidebar.set_show_databases(true);
            self.current_db = None;
            self.status_bar.set_current_db(None);
            self.content.set_content_type(ContentType::Welcome);
            self.content.set_content("MYSQL CLIENT v1.0 - READY\n\n[INSTRUCTIONS]\n- Use Up/Down keys to navigate\n- Press Enter to select database/table\n- Type commands or SQL in bottom input\n- Press 'q' to exit\n\n[STATUS] CONNECTED".to_string());
        }
        Ok(())
    }

    async fn handle_sql_command(&mut self) -> Result<()> {
        let command = self.input.get_input().to_string();
        self.input.clear();
        self.input.set_mode(InputMode::Command);

        if command.trim().is_empty() {
            return Ok(());
        }

        match command.as_str() {
            "\\q" | "\\quit" => {
                return Ok(());
            }
            "\\h" | "\\help" => {
                self.content.set_content_type(ContentType::Help);
                self.content.set_content(self.get_help_content());
            }
            _ => {
                // 执行 SQL 查询
                match self.db_queries.execute_query(&command).await {
                    Ok(results) => {
                        if results.is_empty() {
                            self.content.set_content_type(ContentType::Database);
                            self.content.set_content("查询执行成功，无结果".to_string());
                        } else {
                            // 转换结果为表格格式
                            let headers: Vec<String> = results[0]
                                .as_object()
                                .unwrap()
                                .keys()
                                .map(|k| k.clone())
                                .collect();
                            
                            let rows: Vec<Vec<String>> = results
                                .iter()
                                .map(|row| {
                                    headers
                                        .iter()
                                        .map(|h| {
                                            row.get(h)
                                                .map(|v| v.to_string())
                                                .unwrap_or_else(|| "NULL".to_string())
                                        })
                                        .collect()
                                })
                                .collect();

                            self.content.set_table_data(headers, rows);
                        }
                    }
                    Err(e) => {
                        self.content.set_content_type(ContentType::Error);
                        self.content.set_content(format!("SQL 错误: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    async fn load_databases(&mut self) -> Result<()> {
        let databases = self.db_queries.get_databases().await?;
        self.sidebar.set_databases(databases);
        Ok(())
    }

    async fn load_tables(&mut self) -> Result<()> {
        if let Some(db_name) = &self.current_db {
            match self.db_queries.get_tables(db_name).await {
                Ok(tables) => {
                    self.sidebar.set_tables(tables);
                    self.content.set_content_type(ContentType::Tables);
                    self.content.set_content(format!("数据库 '{}' 的表列表", db_name));
                }
                Err(e) => {
                    self.content.set_content_type(ContentType::Error);
                    self.content.set_content(format!("加载表列表失败: {}", e));
                }
            }
        }
        Ok(())
    }

    async fn load_table_schema(&mut self, table_name: String) -> Result<()> {
        if let Some(db_name) = &self.current_db {
            match self.db_queries.get_table_schema(db_name, &table_name).await {
                Ok((columns, comment)) => {
                    self.content.set_table_name(table_name);
                    self.content.set_table_schema(columns, comment);
                }
                Err(e) => {
                    self.content.set_content_type(ContentType::Error);
                    self.content.set_content(format!("加载表结构失败: {}", e));
                }
            }
        }
        Ok(())
    }

    async fn load_mysql_version(&mut self) -> Result<()> {
        match self.db_queries.get_mysql_version().await {
            Ok(version) => {
                self.status_bar.set_mysql_version(version);
            }
            Err(e) => {
                eprintln!("Failed to get MySQL version: {}", e);
            }
        }
        Ok(())
    }

    fn get_help_content(&self) -> String {
        "帮助信息:\n\n\
        导航:\n\
        - Up/Down: 上下移动选择项\n\
        - Enter: 选择当前项\n\
        - Esc: 返回上一级\n\n\
        快捷键:\n\
        - d: 查看数据库详情\n\
        - t: 查看表详情\n\
        - s: 切换数据库\n\
        - \\: 进入 SQL 模式\n\
        - q: 退出程序\n\n\
        SQL 模式:\n\
        - 输入 SQL 查询语句\n\
        - Enter 执行查询\n\
        - \\q 退出 SQL 模式".to_string()
    }
}
