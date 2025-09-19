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
use std::collections::HashMap;

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
    // 连接配置（用于重建带数据库名的连接池）
    config: Config,
    
    // UI 组件
    sidebar: Sidebar,
    content: Content,
    status_bar: StatusBar,
    input: Input,
    
    // 状态
    current_db: Option<String>,
    // 表名 -> 列名缓存（用于上下文补全）
    table_columns: HashMap<String, Vec<String>>,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let dsn = config.get_dsn();
        let db_connection = DatabaseConnection::new(&dsn).await?;
        let pool = db_connection.get_pool().clone();
        let db_queries = DatabaseQueries::new(pool);

        let mut app = Self {
            db_queries,
            config: config.clone(),
            sidebar: Sidebar::new(),
            content: Content::new(),
            status_bar: StatusBar::new(),
            input: Input::new(),
            current_db: None,
            table_columns: HashMap::new(),
        };

        // 初始化数据
        app.load_databases().await?;
        app.load_mysql_version().await?;
        app.set_username().await?;

        Ok(app)
    }

    async fn rebuild_pool_for_database(&mut self, database_name: Option<String>) -> Result<()> {
        // 更新配置中的数据库名
        self.config.database = database_name;
        let dsn = self.config.get_dsn();
        let db_connection = DatabaseConnection::new(&dsn).await?;
        let pool = db_connection.get_pool().clone();
        self.db_queries = DatabaseQueries::new(pool);
        Ok(())
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

    fn is_at_root(&self) -> bool {
        // 根目录：显示数据库列表且为欢迎页面
        self.sidebar.get_show_databases() && matches!(self.content.get_content_type(), ContentType::Welcome)
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

        // 实时弹出建议浮框：不预留空间，直接覆盖在主内容区底部
        if self.input.get_mode() == &InputMode::SQL && self.input.is_showing_suggestions() {
            let suggestions = self.input.current_suggestions();
            if !suggestions.is_empty() {
                let desired: u16 = std::cmp::min(suggestions.len() as u16 + 2, 8);
                let max_h: u16 = main_chunks[1].height; // 仅覆盖在内容区内部
                let height: u16 = std::cmp::max(1, std::cmp::min(desired, max_h));

                // 根据光标列，计算浮框 x 偏移，尽量靠近光标
                let screen_width = f.area().width as usize;
                let cursor_col = self.input.cursor_display_column();
                let cursor_col_u16 = (cursor_col as u16).min(f.area().width.saturating_sub(10));
                let popup_width: u16 = (screen_width as u16).min(60); // 限宽
                let x = cursor_col_u16.saturating_sub(2).min(main_chunks[1].x + main_chunks[1].width - popup_width);

                let y: u16 = main_chunks[1].y + main_chunks[1].height.saturating_sub(height);
                let popup_area = ratatui::layout::Rect {
                    x,
                    y,
                    width: popup_width,
                    height,
                };
                self.input.render_suggestions_popup(f, popup_area);
            }
        }

        self.input.render(f, chunks[2]);
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        // 如果在SQL模式下，只处理特定的键
        if self.input.get_mode() == &InputMode::SQL {
            match key.code {
                KeyCode::Esc => {
                    // 优先关闭建议框，其次退出 SQL 模式
                    if self.input.is_showing_suggestions() {
                        self.input.hide_suggestions();
                    } else {
                        self.input.set_mode(InputMode::Command);
                    }
                }
                KeyCode::Home => {
                    self.input.move_cursor_start();
                    self.input.hide_suggestions();
                }
                KeyCode::End => {
                    self.input.move_cursor_end();
                    self.input.hide_suggestions();
                }
                KeyCode::Enter => {
                    // Enter 始终执行查询；若有建议，先关闭浮层
                    if self.input.is_showing_suggestions() {
                        self.input.hide_suggestions();
                    }
                    match self.handle_sql_command().await {
                        Ok(should_exit) => {
                            if should_exit {
                                return Ok(true); // 退出程序
                            }
                        }
                        Err(e) => {
                            // SQL 执行失败时显示错误，但不退出程序
                            self.content.set_content_type(ContentType::Error);
                            self.content.set_content(format!("SQL 执行错误: {}", e));
                        }
                    }
                }
                KeyCode::Up => {
                    if self.input.is_showing_suggestions() {
                        self.input.prev_suggestion();
                    } else {
                        // 上箭头键：历史记录向上
                        if let Some(history_command) = self.input.get_history_up() {
                            self.input.clear();
                            for ch in history_command.chars() {
                                self.input.add_char(ch);
                            }
                        }
                    }
                }
                KeyCode::Down => {
                    if self.input.is_showing_suggestions() {
                        self.input.next_suggestion();
                    } else {
                        // 下箭头键：历史记录向下
                        if let Some(history_command) = self.input.get_history_down() {
                            self.input.clear();
                            for ch in history_command.chars() {
                                self.input.add_char(ch);
                            }
                        } else {
                            // 如果到达历史记录末尾，清空输入
                            self.input.clear();
                        }
                    }
                }
                KeyCode::Tab => {
                    // TAB键：应用当前建议；若无建议则尝试生成上下文建议
                    if self.input.is_showing_suggestions() {
                        if let Some(s) = self.input.get_current_suggestion() {
                            self.input.apply_suggestion(&s);
                        }
                    } else {
                        self.update_context_suggestions();
                    }
                }
                KeyCode::Right => {
                    // 单词跳跃（Alt/Option+Right）
                    if key.modifiers.contains(KeyModifiers::ALT) {
                        self.input.move_word_right();
                        self.input.hide_suggestions();
                    } else if self.input.is_showing_suggestions() {
                        // 建议切换
                        self.input.next_suggestion();
                    } else {
                        // 普通光标右移
                        self.input.move_cursor_right();
                        self.input.hide_suggestions();
                    }
                }
                KeyCode::Left => {
                    // 单词跳跃（Alt/Option+Left）
                    if key.modifiers.contains(KeyModifiers::ALT) {
                        self.input.move_word_left();
                        self.input.hide_suggestions();
                    } else if self.input.is_showing_suggestions() {
                        // 建议切换
                        self.input.prev_suggestion();
                    } else {
                        // 普通光标左移
                        self.input.move_cursor_left();
                        self.input.hide_suggestions();
                    }
                }
                KeyCode::Char(ch) => {
                    // 快捷键：行首/行尾、字符/单词跳跃
                    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::SUPER) {
                        match ch {
                            'a' | 'A' => { self.input.move_cursor_start(); }
                            'e' | 'E' => { self.input.move_cursor_end(); }
                            'b' | 'B' => { self.input.move_cursor_left(); }
                            'f' | 'F' => { self.input.move_cursor_right(); }
                            _ => { self.input.add_char(ch); }
                        }
                        // 输入字符后尝试更新上下文建议
                        self.update_context_suggestions();
                    } else if key.modifiers.contains(KeyModifiers::ALT) {
                        match ch {
                            'b' | 'B' => { self.input.move_word_left(); }
                            'f' | 'F' => { self.input.move_word_right(); }
                            _ => { self.input.add_char(ch); }
                        }
                        self.update_context_suggestions();
                    } else {
                        self.input.add_char(ch);
                        // 实时更新上下文建议
                        self.update_context_suggestions();
                    }
                }
                KeyCode::Backspace => {
                    self.input.delete_char();
                    self.update_context_suggestions();
                }
                _ => {
                    // 在SQL模式下忽略其他所有键
                }
            }
            return Ok(false);
        }

        // 在CMD模式下处理所有快捷键
        match key.code {
            KeyCode::Char('q') => {
                // 仅在根目录退出；其他情况下等价于 Esc 返回上一级
                if self.is_at_root() {
                    return Ok(true);
                } else {
                    self.handle_escape().await?;
                }
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
                // 根据内容类型处理滚动
                match self.content.get_content_type() {
                    ContentType::TableSchema => {
                        self.content.scroll_schema_up();
                    }
                    ContentType::TableData => {
                        self.content.scroll_data_up();
                    }
                    _ => {
                        self.sidebar.previous_item();
                    }
                }
            }
            KeyCode::Down => {
                // 根据内容类型处理滚动
                match self.content.get_content_type() {
                    ContentType::TableSchema => {
                        self.content.scroll_schema_down();
                    }
                    ContentType::TableData => {
                        self.content.scroll_data_down();
                    }
                    _ => {
                        self.sidebar.next_item();
                    }
                }
            }
            KeyCode::Left => {
                // 如果在表数据模式下，处理水平滚动
                if matches!(self.content.get_content_type(), ContentType::TableData) {
                    self.content.scroll_data_left();
                }
            }
            KeyCode::Right => {
                // 如果在表数据模式下，处理水平滚动
                if matches!(self.content.get_content_type(), ContentType::TableData) {
                    self.content.scroll_data_right();
                }
            }
            KeyCode::Enter => {
                self.handle_enter().await?;
            }
            KeyCode::Char(' ') => {
                // 在SQL模式下，空格键直接添加到输入中
                if self.input.get_mode() == &InputMode::SQL {
                    self.input.add_char(' ');
                } else {
                    self.handle_space().await?;
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
                KeyCode::Char(':') => {
                    // 进入SQL模式
                    self.input.set_mode(InputMode::SQL);
                    // 更新当前数据库信息
                    self.input.set_current_db(self.current_db.clone());
                    // 重置历史记录索引
                    self.input.reset_history_index();
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
                self.content.set_content("MYSQL CLIENT v1.0 - READY\n\n[INSTRUCTIONS]\n- Use Up/Down keys to navigate\n- Press Enter to view table structure\n- Press Space to view table data (10 rows)\n- Press ':' to enter SQL edit mode\n- Press 'q' to exit\n\n[STATUS] CONNECTED".to_string());
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
                self.content.set_content(format!("正在切换到数据库 '{}'...", db_name));
                
                // 重建连接池以设置默认数据库，避免 USE 的预处理限制
                if let Err(e) = self.rebuild_pool_for_database(Some(db_name.clone())).await {
                    self.content.set_content_type(ContentType::Error);
                    self.content.set_content(format!("切换数据库失败: {}", e));
                    return Ok(());
                }
                
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
                self.content.reset_schema_scroll(); // 重置滚动位置
                if let Err(e) = self.load_table_schema(table_name).await {
                    self.content.set_content_type(ContentType::Error);
                    self.content.set_content(format!("加载表结构失败: {}", e));
                }
            }
        }
        Ok(())
    }

    fn update_context_suggestions(&mut self) {
        if self.input.get_mode() != &InputMode::SQL { return; }
        let input = self.input.get_input();
        let cursor_pos = self.input.get_cursor_pos();

        // 获取光标左侧 token
        let chars: Vec<char> = input.chars().collect();
        let mut i = cursor_pos;
        while i > 0 && chars[i-1].is_whitespace() { i -= 1; }
        let mut start = i;
        while start > 0 && (chars[start-1].is_alphanumeric() || chars[start-1] == '_' || chars[start-1] == '$' || chars[start-1] == '.') { start -= 1; }
        let token: String = chars[start..i].iter().collect();
        let before: String = chars[..start].iter().collect();
        let before_lower = before.to_lowercase();

        // 规则：
        // use -> 数据库列表；
        // from/join/desc/describe -> 表列表；
        // where/and/or/<table>. -> 列名；
        // 默认 -> SQL 关键字
        if before_lower.ends_with("use ") {
            // 建议数据库
            let dbs: Vec<String> = self.sidebar
                .get_databases_ref()
                .iter()
                .map(|d| d.name.clone())
                .filter(|name| token.is_empty() || name.to_lowercase().starts_with(&token.to_lowercase()))
                .collect();
            self.input.set_external_suggestions(dbs);
            return;
        }

        // 检测关键字后的空格：from / join / desc / describe
        let triggers = ["from ", "join ", "desc ", "describe "];
        if triggers.iter().any(|t| before_lower.ends_with(t)) {
            let tables: Vec<String> = self.sidebar
                .get_tables_ref()
                .iter()
                .map(|t| t.name.clone())
                .filter(|name| token.is_empty() || name.to_lowercase().starts_with(&token.to_lowercase()))
                .collect();
            self.input.set_external_suggestions(tables);
            return;
        }

        // WHERE/AND/OR 上下文：建议列名（若能解析到表名，优先对应表；否则合并当前库所有表的列）
        let where_triggers = ["where ", "and ", "or "]; // 简化处理
        let is_where_context = where_triggers.iter().any(|t| before_lower.ends_with(t));
        let dot_context = token.ends_with('.');
        if is_where_context || dot_context {
            let mut column_suggestions: Vec<String> = Vec::new();
            // 简化：如果 token 包含 table. 前缀，则只取对应表的列
            let table_prefix_opt = if dot_context {
                let tbl = token.trim_end_matches('.');
                if !tbl.is_empty() { Some(tbl.to_string()) } else { None }
            } else { None };

            if let Some(tbl) = table_prefix_opt {
                if let Some(cols) = self.table_columns.get(&tbl) {
                    column_suggestions = cols.clone();
                } else {
                    // 未缓存则暂不加载（避免阻塞/引入依赖）；清空建议
                    column_suggestions.clear();
                }
                self.input.set_external_suggestions(column_suggestions);
                return;
            }

            // 未提供 table. 前缀，则合并当前已知表的列（去重）
            let mut set = std::collections::BTreeSet::new();
            for cols in self.table_columns.values() {
                for c in cols { set.insert(c.clone()); }
            }
            column_suggestions = set.into_iter().collect();
            if !column_suggestions.is_empty() {
                // 过滤前缀匹配
                let prefix = token.to_lowercase();
                let filtered: Vec<String> = column_suggestions.into_iter()
                    .filter(|c| prefix.is_empty() || c.to_lowercase().starts_with(&prefix))
                    .collect();
                self.input.set_external_suggestions(filtered);
                return;
            }
        }

        // 默认清空外部建议，回退到内建 SQL 关键字建议（仅在空输入时显示）
        self.input.clear_external_suggestions();
        if token.is_empty() {
            self.input.show_suggestions();
        } else {
            self.input.hide_suggestions();
        }
    }

    async fn handle_space(&mut self) -> Result<()> {
        if !self.sidebar.get_show_databases() {
            if let Some(table) = self.sidebar.get_selected_table() {
                let table_name = table.name.clone();
                self.content.set_content_type(ContentType::TableData);
                self.content.set_content("正在加载表数据...".to_string());
                self.content.reset_data_scroll(); // 重置数据滚动位置
                if let Err(e) = self.load_table_data(table_name, 10).await {
                    self.content.set_content_type(ContentType::Error);
                    self.content.set_content(format!("加载表数据失败: {}", e));
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
            self.content.set_content("MYSQL CLIENT v1.0 - READY\n\n[INSTRUCTIONS]\n- Use Up/Down keys to navigate\n- Press Enter to view table structure\n- Press Space to view table data (10 rows)\n- Press ':' to enter SQL edit mode\n- Press 'q' to exit\n\n[STATUS] CONNECTED".to_string());
        }
        Ok(())
    }

    async fn handle_sql_command(&mut self) -> Result<bool> {
        let raw_command = self.input.get_input().to_string();
        
        // 添加到历史记录
        self.input.add_to_history(raw_command.clone());
        
        self.input.clear();
        // 保持在 SQL 模式，直到用户按 Esc 主动退出

        if raw_command.trim().is_empty() {
            return Ok(false);
        }

        // \G 兼容：检测并移除末尾的 \G / \g（大小写与空白兼容）
        let mut use_vertical = false;
        let mut command = raw_command.clone();
        {
            let trimmed = raw_command.trim_end();
            // 兼容尾随 ; 与空白：如  "SELECT 1;  \\G" 或 "SELECT 1 \\g"
            // 先去掉末尾空白
            let mut end = trimmed.len();
            // 反向跳过空白
            while end > 0 && trimmed.as_bytes()[end - 1].is_ascii_whitespace() { end -= 1; }
            // 处理可选的 ;
            if end > 0 && trimmed.as_bytes()[end - 1] == b';' {
                end -= 1;
                while end > 0 && trimmed.as_bytes()[end - 1].is_ascii_whitespace() { end -= 1; }
            }
            // 尝试匹配以 \\G 或 \\g 结尾
            if end >= 2 {
                let tail = &trimmed[end - 2..end];
                if tail == "\\G" || tail == "\\g" {
                    use_vertical = true;
                    // 去除尾部标记并还原命令
                    let base = &trimmed[..end - 2].trim_end();
                    command = base.trim_end_matches(';').trim_end().to_string();
                }
            }
        }

        // 检查是否是USE命令
        if let Some(db_name) = self.parse_use_command(&command) {
            self.handle_use_database(db_name).await?;
            return Ok(false);
        }

        match command.as_str() {
            "\\h" | "\\help" => {
                self.content.set_content_type(ContentType::Help);
                self.content.set_content(self.get_help_content());
            }
            "exit" | "quit" | "\\q" | "\\quit" => {
                // 退出程序
                return Ok(true);
            }
            _ => {
                // 根据首个关键字判断是查询类还是非查询类
                let first_word = command
                    .trim_start()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_uppercase();

                let is_query = matches!(
                    first_word.as_str(),
                    "SELECT" | "SHOW" | "DESCRIBE" | "DESC" | "EXPLAIN"
                );

                if is_query {
                    match self.db_queries.execute_query_raw(&command).await {
                        Ok((headers, rows)) => {
                            if rows.is_empty() {
                                self.content.set_content_type(ContentType::Database);
                                self.content.set_content("查询执行成功，无结果".to_string());
                            } else {
                                if use_vertical {
                                    self.content.set_table_data_vertical(headers, rows);
                                } else {
                                    self.content.set_table_data(headers, rows);
                                }
                            }
                        }
                        Err(e) => {
                            self.content.set_content_type(ContentType::Error);
                            self.content.set_content(format!("SQL 错误: {}", e));
                        }
                    }
                } else {
                    match self.db_queries.execute_non_query(&command).await {
                        Ok(affected) => {
                            self.content.set_content_type(ContentType::Database);
                            self.content.set_content(format!("执行成功，受影响行数: {}", affected));
                        }
                        Err(e) => {
                            self.content.set_content_type(ContentType::Error);
                            self.content.set_content(format!("SQL 错误: {}", e));
                        }
                    }
                }
            }
        }
        Ok(false)
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
                    // 先写入缓存再更新 UI
                    let col_names: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
                    self.table_columns.insert(table_name.clone(), col_names);
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

    async fn load_table_data(&mut self, table_name: String, limit: usize) -> Result<()> {
        if let Some(_db_name) = &self.current_db {
            // 由于已经执行了 USE 命令，可以直接使用表名
            let query = format!("SELECT * FROM `{}` LIMIT {}", table_name, limit);
            match self.db_queries.execute_query_raw(&query).await {
                Ok((headers, rows)) => {
                    if rows.is_empty() {
                        self.content.set_content_type(ContentType::TableData);
                        self.content.set_content("表为空，没有数据".to_string());
                    } else {
                        self.content.set_table_data(headers, rows);
                    }
                }
                Err(e) => {
                    self.content.set_content_type(ContentType::Error);
                    self.content.set_content(format!("加载表数据失败: {}", e));
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

    async fn set_username(&mut self) -> Result<()> {
        match self.db_queries.get_current_user().await {
            Ok(username) => {
                self.status_bar.set_username(username);
            }
            Err(e) => {
                eprintln!("Failed to get current user: {}", e);
            }
        }
        Ok(())
    }

    fn parse_use_command(&self, command: &str) -> Option<String> {
        let trimmed = command.trim();
        // 支持多种格式：USE db, use db, USE db;, use db; 等
        if trimmed.to_uppercase().starts_with("USE ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                let db_name = parts[1].trim_end_matches(';').trim();
                if !db_name.is_empty() {
                    return Some(db_name.to_string());
                }
            }
        }
        None
    }

    async fn handle_use_database(&mut self, db_name: String) -> Result<()> {
        // 检查数据库是否存在
        let databases = self.db_queries.get_databases().await?;
        if !databases.iter().any(|db| db.name == db_name) {
            self.content.set_content_type(ContentType::Error);
            self.content.set_content(format!("数据库 '{}' 不存在", db_name));
            return Ok(());
        }

        // 重建连接池到目标数据库，避免 USE 的预处理限制
        if let Err(e) = self.rebuild_pool_for_database(Some(db_name.clone())).await {
            self.content.set_content_type(ContentType::Error);
            self.content.set_content(format!("切换数据库失败: {}", e));
            return Ok(());
        }
        
        // 切换数据库
        self.current_db = Some(db_name.clone());
        self.status_bar.set_current_db(Some(db_name.clone()));
        self.sidebar.set_show_databases(false);
        self.sidebar.set_current_db(Some(db_name.clone()));
        
        // 更新输入组件的数据库信息
        self.input.set_current_db(Some(db_name.clone()));
        
        // 加载新数据库的表
        self.content.set_content_type(ContentType::Database);
        self.content.set_content(format!("已切换到数据库 '{}'，正在加载表...", db_name));
        
        if let Err(e) = self.load_tables().await {
            self.content.set_content_type(ContentType::Error);
            self.content.set_content(format!("加载表列表失败: {}", e));
        } else {
            self.content.set_content_type(ContentType::Database);
            self.content.set_content(format!("已切换到数据库 '{}'，共 {} 个表", db_name, self.sidebar.get_tables_count()));
        }
        
        Ok(())
    }

    fn get_help_content(&self) -> String {
        "帮助信息:\n\n\
        导航:\n\
        - Up/Down: 上下移动选择项\n\
        - Enter: 查看表结构\n\
        - Space: 查看表数据(前10行)\n\
        - Esc: 返回上一级\n\n\
        快捷键:\n\
        - d: 查看数据库详情\n\
        - t: 查看表详情\n\
        - s: 切换数据库\n\
        - : 进入 SQL 编辑模式\n\
        - q: 退出程序\n\n\
        SQL 编辑模式:\n\
        - 输入 SQL 查询语句\n\
        - Enter 执行查询\n\
        - Tab 添加缩进(4个空格)\n\
        - 在查询末尾添加 \\\\G 使用垂直输出\n\
        - USE database 切换数据库\n\
        - exit/quit/\\q 退出程序\n\
        - Esc 退出 SQL 编辑模式\n\n\
        表结构模式:\n\
        - Up/Down: 滚动查看字段\n\
        - Esc: 返回表列表\n\n\
        表数据模式:\n\
        - Up/Down: 垂直滚动查看行（垂直输出时切换行）\n\
        - Left/Right: 水平滚动查看列\n\
        - Esc: 返回表列表".to_string()
    }
}
