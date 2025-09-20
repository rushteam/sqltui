use anyhow::Result;
use async_trait::async_trait;
use sqlx::{MySql, Pool, Row, Column};

use crate::models::{Database, Table, SchemaColumn};
use crate::db::adapter::DbAdapter;

pub struct MySqlAdapter {
    pool: Pool<MySql>,
}

impl MySqlAdapter {
    pub async fn new(dsn: &str) -> Result<Self> {
        let pool = sqlx::MySqlPool::connect(dsn).await?;
        // 连接后设置字符集
        sqlx::query("SET NAMES utf8mb4 COLLATE utf8mb4_unicode_ci").execute(&pool).await?;
        sqlx::query("SET character_set_client=utf8mb4").execute(&pool).await?;
        sqlx::query("SET character_set_connection=utf8mb4").execute(&pool).await?;
        sqlx::query("SET character_set_results=utf8mb4").execute(&pool).await?;
        Ok(Self { pool })
    }

    fn get_cell_value_as_string(row: &sqlx::mysql::MySqlRow, index: usize) -> String {
        if let Ok(v) = row.try_get::<String, _>(index) { return v; }
        if let Ok(v) = row.try_get::<i64, _>(index) { return v.to_string(); }
        if let Ok(v) = row.try_get::<f64, _>(index) { return v.to_string(); }
        if let Ok(v) = row.try_get::<bool, _>(index) { return if v { "1".into() } else { "0".into() }; }
        if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(index) { return v.format("%Y-%m-%d %H:%M:%S").to_string(); }
        if let Ok(v) = row.try_get::<chrono::NaiveDate, _>(index) { return v.format("%Y-%m-%d").to_string(); }
        if let Ok(v) = row.try_get::<chrono::NaiveTime, _>(index) { return v.format("%H:%M:%S").to_string(); }
        if let Ok(v) = row.try_get::<Vec<u8>, _>(index) { return String::from_utf8_lossy(&v).to_string(); }
        if let Ok(v) = row.try_get::<serde_json::Value, _>(index) { return v.to_string(); }
        "NULL".into()
    }
}

#[async_trait]
impl DbAdapter for MySqlAdapter {
    fn driver_name(&self) -> &'static str { "MySQL" }
    fn keywords(&self) -> &'static [&'static str] {
        &[
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP",
            "ALTER", "USE", "SHOW", "DESCRIBE", "EXPLAIN", "JOIN", "LEFT", "RIGHT", "INNER",
            "OUTER", "ON", "GROUP", "BY", "ORDER", "HAVING", "LIMIT", "OFFSET", "DISTINCT",
            "COUNT", "SUM", "AVG", "MIN", "MAX", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN",
            "IS", "NULL", "TRUE", "FALSE", "ASC", "DESC", "AS", "UNION", "ALL", "EXISTS",
            "DATABASES", "TABLES", "COLUMNS", "INDEX", "INDEXES", "PROCESSLIST", "STATUS",
            "VARIABLES", "GRANTS", "PRIVILEGES", "USERS", "FUNCTIONS", "PROCEDURES", "TRIGGERS"
        ]
    }
    fn system_databases(&self) -> &'static [&'static str] {
        &["information_schema", "performance_schema", "mysql", "sys"]
    }
    fn supports_use_database(&self) -> bool { true }
    fn quote_ident(&self, ident: &str) -> String { format!("`{}`", ident.replace('`', "``")) }

    async fn get_databases(&self) -> Result<Vec<Database>> {
        let rows = sqlx::query("SHOW DATABASES").fetch_all(&self.pool).await?;
        let mut databases = Vec::new();
        for row in rows {
            let db_name = Self::get_cell_value_as_string(&row, 0);
            if ["information_schema","performance_schema","mysql","sys"].contains(&db_name.as_str()) { continue; }
            // 尝试获取表数量（可能失败，但不影响基本功能）
            let count = sqlx::query(&format!("SHOW TABLES FROM `{}`", db_name))
                .fetch_all(&self.pool)
                .await
                .map(|v| v.len() as u64)
                .ok();
            databases.push(Database::with_details(db_name, None, None, count));
        }
        Ok(databases)
    }

    async fn get_tables(&self, database_name: &str) -> Result<Vec<Table>> {
        let query = format!("SHOW TABLES FROM `{}`", database_name);
        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;
        let mut tables = Vec::new();
        for row in rows {
            let table_name = match row.try_get::<String, _>(0) {
                Ok(name) => name,
                Err(_) => String::from_utf8_lossy(&row.get::<Vec<u8>, _>(0)).to_string(),
            };
            tables.push(Table::with_details(table_name, None, None, None, None));
        }
        Ok(tables)
    }

    async fn get_table_schema(&self, database_name: &str, table_name: &str) -> Result<(Vec<SchemaColumn>, Option<String>)> {
        // 详尽信息
        let table_comment = sqlx::query(
            "SELECT TABLE_COMMENT as comment FROM information_schema.TABLES WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?"
        )
        .bind(database_name)
        .bind(table_name)
        .fetch_optional(&self.pool)
        .await?
        .and_then(|row| {
            let comment: String = String::from_utf8_lossy(&row.get::<Vec<u8>, _>("comment")).to_string();
            if comment.is_empty() { None } else { Some(comment) }
        });

        let rows = sqlx::query(
            r#"
            SELECT 
                COLUMN_NAME as name,
                DATA_TYPE as data_type,
                IS_NULLABLE as is_nullable,
                COLUMN_DEFAULT as default_value,
                EXTRA as extra,
                COLUMN_COMMENT as comment
            FROM information_schema.COLUMNS 
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            ORDER BY ORDINAL_POSITION
            "#
        )
        .bind(database_name)
        .bind(table_name)
        .fetch_all(&self.pool)
        .await?;

        let columns = rows.into_iter().map(|row| {
            let is_nullable: String = String::from_utf8_lossy(&row.get::<Vec<u8>, _>("is_nullable")).to_string();
            let default_value: Option<String> = match row.try_get::<String, _>("default_value") {
                Ok(val) => Some(val),
                Err(_) => row.try_get::<Vec<u8>, _>("default_value").ok().map(|b| String::from_utf8_lossy(&b).to_string()),
            };
            let extra: String = String::from_utf8_lossy(&row.get::<Vec<u8>, _>("extra")).to_string();
            let comment: String = String::from_utf8_lossy(&row.get::<Vec<u8>, _>("comment")).to_string();
            SchemaColumn::with_details(
                String::from_utf8_lossy(&row.get::<Vec<u8>, _>("name")).to_string(),
                String::from_utf8_lossy(&row.get::<Vec<u8>, _>("data_type")).to_string(),
                is_nullable == "YES",
                default_value,
                if extra.is_empty() { None } else { Some(extra) },
                if comment.is_empty() { None } else { Some(comment) },
            )
        }).collect();

        Ok((columns, table_comment))
    }

    async fn execute_query_raw(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;
        if rows.is_empty() { return Ok((Vec::new(), Vec::new())); }
        let headers: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
        let mut data_rows = Vec::new();
        for row in rows { 
            let mut row_data = Vec::new();
            for i in 0..row.columns().len() { row_data.push(Self::get_cell_value_as_string(&row, i)); }
            data_rows.push(row_data);
        }
        Ok((headers, data_rows))
    }

    async fn execute_non_query(&self, query: &str) -> Result<u64> {
        let result = sqlx::query(query).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    async fn get_version(&self) -> Result<String> {
        let row = sqlx::query("SELECT VERSION() as version").fetch_one(&self.pool).await?;
        Ok(row.get::<String, _>("version"))
    }

    async fn get_current_user(&self) -> Result<String> {
        let row = sqlx::query("SELECT USER() as user").fetch_one(&self.pool).await?;
        Ok(row.get::<String, _>("user"))
    }
}


