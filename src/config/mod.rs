use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// 数据库驱动: mysql | pgsql | clickhouse
    #[arg(long, value_parser = ["mysql", "pgsql", "clickhouse"], default_value = "mysql")]
    pub driver: String,
    /// MySQL host
    #[arg(short = 'H', long, default_value = "localhost")]
    pub host: String,

    /// MySQL port
    #[arg(short = 'P', long, default_value = "3306")]
    pub port: u16,

    /// MySQL username
    #[arg(short = 'u', long, default_value = "root")]
    pub username: String,

    /// MySQL password
    #[arg(short = 'p', long, default_value = "")]
    pub password: String,

    /// MySQL database
    #[arg(short = 'd', long)]
    pub database: Option<String>,
}

impl Config {
    pub fn driver(&self) -> Driver {
        match self.driver.as_str() {
            "mysql" => Driver::Mysql,
            "pgsql" => Driver::Postgres,
            "clickhouse" => Driver::Clickhouse,
            _ => Driver::Mysql,
        }
    }

    pub fn get_dsn(&self) -> String {
        if self.password.is_empty() {
            // 如果没有密码，不包含密码部分
            match self.driver() {
                Driver::Mysql => format!(
                    "mysql://{}@{}:{}/{}?charset=utf8mb4&collation=utf8mb4_unicode_ci",
                    self.username,
                    self.host,
                    self.port,
                    self.database.as_deref().unwrap_or("")
                ),
                Driver::Postgres => format!(
                    "postgres://{}@{}:{}/{}",
                    self.username,
                    self.host,
                    self.port,
                    self.database.as_deref().unwrap_or("")
                ),
                Driver::Clickhouse => format!(
                    "clickhouse://{}@{}:{}/{}",
                    self.username,
                    self.host,
                    self.port,
                    self.database.as_deref().unwrap_or("")
                ),
            }
        } else {
            match self.driver() {
                Driver::Mysql => format!(
                    "mysql://{}:{}@{}:{}/{}?charset=utf8mb4&collation=utf8mb4_unicode_ci",
                    self.username,
                    self.password,
                    self.host,
                    self.port,
                    self.database.as_deref().unwrap_or("")
                ),
                Driver::Postgres => format!(
                    "postgres://{}:{}@{}:{}/{}",
                    self.username,
                    self.password,
                    self.host,
                    self.port,
                    self.database.as_deref().unwrap_or("")
                ),
                Driver::Clickhouse => format!(
                    "clickhouse://{}:{}@{}:{}/{}",
                    self.username,
                    self.password,
                    self.host,
                    self.port,
                    self.database.as_deref().unwrap_or("")
                ),
            }
        }
    }

    pub fn get_connection_info(&self) -> (String, String, u16) {
        (self.username.clone(), self.host.clone(), self.port)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Driver {
    Mysql,
    Postgres,
    Clickhouse,
}
