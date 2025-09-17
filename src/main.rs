use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::io::{self, Write};

mod config;
mod db;
mod models;
mod ui;

use clap::Parser;
use config::Config;
use ui::App;

// 全局 panic 处理器
fn setup_panic_handler() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // 恢复终端状态
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen
        );
        let _ = io::stdout().flush();
        
        // 调用原始的 panic 处理器
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() -> Result<()> {
    // 设置 panic 处理器
    setup_panic_handler();
    
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // 解析命令行参数
    let config = Config::parse();
    
    // 获取连接信息
    let (_user, host, port) = config.get_connection_info();
    // info!("正在连接到 MySQL 服务器 {}:{}", host, port);

    // 创建并运行应用
    let mut app = App::new(config).await?;
    // info!("成功连接到 MySQL 服务器 {}:{}", host, port);
    
    // 运行 TUI
    app.run().await?;

    Ok(())
}