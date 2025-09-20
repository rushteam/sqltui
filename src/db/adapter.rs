use anyhow::{Result, anyhow};
use async_trait::async_trait;

use crate::{config::{Config, Driver}, models::{Database, Table, SchemaColumn}};

use super::{connection::DatabaseConnection, queries::DatabaseQueries};

#[async_trait]
pub trait DbAdapter: Send + Sync {
    async fn get_databases(&self) -> Result<Vec<Database>>;
    async fn get_tables(&self, database_name: &str) -> Result<Vec<Table>>;
    async fn get_table_schema(&self, database_name: &str, table_name: &str) -> Result<(Vec<SchemaColumn>, Option<String>)>;
    async fn execute_query_raw(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)>;
    async fn execute_non_query(&self, query: &str) -> Result<u64>;
    async fn get_version(&self) -> Result<String>;
    async fn get_current_user(&self) -> Result<String>;
}

pub struct MySqlAdapter {
    queries: DatabaseQueries,
}

impl MySqlAdapter {
    pub async fn new(dsn: &str) -> Result<Self> {
        let conn = DatabaseConnection::new(dsn).await?;
        let pool = conn.get_pool().clone();
        Ok(Self { queries: DatabaseQueries::new(pool) })
    }
}

#[async_trait]
impl DbAdapter for MySqlAdapter {
    async fn get_databases(&self) -> Result<Vec<Database>> { self.queries.get_databases().await }
    async fn get_tables(&self, database_name: &str) -> Result<Vec<Table>> { self.queries.get_tables(database_name).await }
    async fn get_table_schema(&self, database_name: &str, table_name: &str) -> Result<(Vec<SchemaColumn>, Option<String>)> { self.queries.get_table_schema(database_name, table_name).await }
    async fn execute_query_raw(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> { self.queries.execute_query_raw(query).await }
    async fn execute_non_query(&self, query: &str) -> Result<u64> { self.queries.execute_non_query(query).await }
    async fn get_version(&self) -> Result<String> { self.queries.get_mysql_version().await }
    async fn get_current_user(&self) -> Result<String> { self.queries.get_current_user().await }
}

pub async fn new_adapter(config: &Config) -> Result<Box<dyn DbAdapter>> {
    let dsn = config.get_dsn();
    match config.driver() {
        Driver::Mysql => Ok(Box::new(MySqlAdapter::new(&dsn).await?)),
        Driver::Postgres => Err(anyhow!("Postgres 适配器暂未实现")),
        Driver::Clickhouse => Err(anyhow!("ClickHouse 适配器暂未实现")),
    }
}


