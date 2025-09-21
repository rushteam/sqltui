use anyhow::Result;
use async_trait::async_trait;
use sqlx::{Pool, Postgres, Row, Column};

use crate::db::adapter::DbAdapter;
use crate::models::{Database, SchemaColumn, Table};

pub struct PostgresAdapter {
    pool: Pool<Postgres>,
}

impl PostgresAdapter {
    pub async fn new(dsn: &str) -> Result<Self> {
        let pool = sqlx::PgPool::connect(dsn).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl DbAdapter for PostgresAdapter {
    fn driver_name(&self) -> &'static str { "PostgreSQL" }

    fn keywords(&self) -> &'static [&'static str] {
        &[
            "SELECT","FROM","WHERE","INSERT","UPDATE","DELETE","CREATE","DROP",
            "ALTER","SHOW","EXPLAIN","JOIN","LEFT","RIGHT","INNER","OUTER","ON",
            "GROUP","BY","ORDER","HAVING","LIMIT","OFFSET","DISTINCT","COUNT","SUM",
            "AVG","MIN","MAX","AND","OR","NOT","IN","LIKE","BETWEEN","IS","NULL",
            "TRUE","FALSE","ASC","DESC","AS","UNION","ALL","EXISTS","TABLE","SCHEMA"
        ]
    }

    fn system_databases(&self) -> &'static [&'static str] { &["postgres", "template0", "template1"] }

    fn supports_use_database(&self) -> bool { false }

    fn quote_ident(&self, ident: &str) -> String { format!("\"{}\"", ident.replace('"', "\"\"")) }

    async fn get_databases(&self) -> Result<Vec<Database>> {
        let rows = sqlx::query(
            "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname"
        ).fetch_all(&self.pool).await?;
        let mut v = Vec::new();
        for row in rows {
            let name: String = row.try_get::<String, _>("datname")?;
            v.push(Database::with_details(name, None, None, None));
        }
        Ok(v)
    }

    async fn get_tables(&self, database_name: &str) -> Result<Vec<Table>> {
        // 在 PostgreSQL 中，表属于 schema。默认使用 public schema。
        let _ = database_name; // 已通过 DSN 指定数据库
        let rows = sqlx::query(
            r#"
            SELECT tablename AS name
            FROM pg_catalog.pg_tables
            WHERE schemaname = 'public'
            ORDER BY tablename
            "#
        ).fetch_all(&self.pool).await?;
        let mut v = Vec::new();
        for row in rows {
            let name: String = row.try_get::<String, _>("name").unwrap_or_else(|_| "".to_string());
            v.push(Table::with_details(name, None, None, None, None));
        }
        Ok(v)
    }

    async fn get_table_schema(&self, _database_name: &str, table_name: &str) -> Result<(Vec<SchemaColumn>, Option<String>)> {
        let comment_row = sqlx::query(
            r#"
            SELECT obj_description(pg_class.oid) AS comment
            FROM pg_class
            JOIN pg_namespace ON pg_namespace.oid = pg_class.relnamespace
            WHERE pg_class.relkind = 'r' AND pg_namespace.nspname = 'public' AND pg_class.relname = $1
            "#
        )
        .bind(table_name)
        .fetch_optional(&self.pool)
        .await?;
        let table_comment: Option<String> = comment_row
            .and_then(|row| row.try_get::<String, _>("comment").ok())
            .and_then(|s| if s.is_empty() { None } else { Some(s) });

        let rows = sqlx::query(
            r#"
            SELECT
                a.attname AS name,
                pg_catalog.format_type(a.atttypid, a.atttypmod) AS data_type,
                NOT a.attnotnull AS is_nullable,
                pg_get_expr(ad.adbin, ad.adrelid) AS default_value,
                col_description(a.attrelid, a.attnum) AS comment
            FROM pg_attribute a
            JOIN pg_class c ON a.attrelid = c.oid
            JOIN pg_type t ON a.atttypid = t.oid
            LEFT JOIN pg_attrdef ad ON a.attrelid = ad.adrelid AND a.attnum = ad.adnum
            WHERE a.attnum > 0 AND NOT a.attisdropped
              AND c.relname = $1
              AND c.relkind = 'r'
            ORDER BY a.attnum
            "#
        )
        .bind(table_name)
        .fetch_all(&self.pool)
        .await?;

        let mut cols = Vec::new();
        for row in rows {
            let name: String = row.try_get::<String, _>("name").unwrap_or_default();
            let data_type: String = row.try_get::<String, _>("data_type").unwrap_or_default();
            let is_nullable: bool = row.try_get::<bool, _>("is_nullable").unwrap_or(true);
            let default_value: Option<String> = row.try_get::<String, _>("default_value").ok();
            let comment: Option<String> = row.try_get::<String, _>("comment").ok();
            cols.push(SchemaColumn::with_details(name, data_type, is_nullable, default_value, None, comment));
        }
        Ok((cols, table_comment))
    }

    async fn execute_query_raw(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;
        if rows.is_empty() { return Ok((Vec::new(), Vec::new())); }
        let cols = rows[0].columns();
        let headers: Vec<String> = cols.iter().map(|c| c.name().to_string()).collect();
        let mut data_rows = Vec::new();
        for row in rows {
            let mut r = Vec::new();
            for (idx, _c) in row.columns().iter().enumerate() {
                // 尝试多种常见类型转字符串
                if let Ok(v) = row.try_get::<String, _>(idx) { r.push(v); continue; }
                if let Ok(v) = row.try_get::<i64, _>(idx) { r.push(v.to_string()); continue; }
                if let Ok(v) = row.try_get::<f64, _>(idx) { r.push(v.to_string()); continue; }
                if let Ok(v) = row.try_get::<bool, _>(idx) { r.push((if v {"1"} else {"0"}).to_string()); continue; }
                if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(idx) { r.push(v.format("%Y-%m-%d %H:%M:%S").to_string()); continue; }
                if let Ok(v) = row.try_get::<chrono::NaiveDate, _>(idx) { r.push(v.format("%Y-%m-%d").to_string()); continue; }
                if let Ok(v) = row.try_get::<chrono::NaiveTime, _>(idx) { r.push(v.format("%H:%M:%S").to_string()); continue; }
                if let Ok(v) = row.try_get::<serde_json::Value, _>(idx) { r.push(v.to_string()); continue; }
                r.push("NULL".to_string());
            }
            data_rows.push(r);
        }
        Ok((headers, data_rows))
    }

    async fn execute_non_query(&self, query: &str) -> Result<u64> {
        let result = sqlx::query(query).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    async fn get_version(&self) -> Result<String> {
        let row = sqlx::query("SELECT version() AS version").fetch_one(&self.pool).await?;
        let v: String = row.try_get("version")?;
        Ok(v)
    }

    async fn get_current_user(&self) -> Result<String> {
        // SHOW user 在 PG 不适用，使用 current_user
        let row = sqlx::query("SELECT current_user AS usr").fetch_one(&self.pool).await?;
        let u: String = row.try_get("usr")?;
        Ok(u)
    }
}

