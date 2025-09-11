use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct Config {
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
    pub fn get_dsn(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}?charset=utf8mb4&collation=utf8mb4_unicode_ci",
            self.username,
            self.password,
            self.host,
            self.port,
            self.database.as_deref().unwrap_or("")
        )
    }

    pub fn get_connection_info(&self) -> (String, String, u16) {
        (self.username.clone(), self.host.clone(), self.port)
    }
}
