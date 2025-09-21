use anyhow::{Result, anyhow};
use async_trait::async_trait;

use crate::{config::{Config, Driver}, models::{Database, Table, SchemaColumn}};

use crate::db::adapters::mysql::MySqlAdapter;
use crate::db::adapters::postgres::PostgresAdapter;

#[async_trait]
pub trait DbAdapter: Send + Sync {
    fn driver_name(&self) -> &'static str;
    fn keywords(&self) -> &'static [&'static str];
    fn system_databases(&self) -> &'static [&'static str];
    fn supports_use_database(&self) -> bool { true }
    fn quote_ident(&self, ident: &str) -> String { format!("`{}`", ident.replace('`', "``")) }
    async fn get_databases(&self) -> Result<Vec<Database>>;
    async fn get_tables(&self, database_name: &str) -> Result<Vec<Table>>;
    async fn get_table_schema(&self, database_name: &str, table_name: &str) -> Result<(Vec<SchemaColumn>, Option<String>)>;
    async fn execute_query_raw(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>) >;
    async fn execute_non_query(&self, query: &str) -> Result<u64>;
    async fn get_version(&self) -> Result<String>;
    async fn get_current_user(&self) -> Result<String>;
}

pub async fn new_adapter(config: &Config) -> Result<Box<dyn DbAdapter>> {
    let dsn = config.get_dsn();
    match config.driver() {
        Driver::Mysql => Ok(Box::new(MySqlAdapter::new(&dsn).await?)),
        Driver::Postgres => Ok(Box::new(PostgresAdapter::new(&dsn).await?)),
        Driver::Clickhouse => Err(anyhow!("ClickHouse 适配器暂未实现")),
    }
}


