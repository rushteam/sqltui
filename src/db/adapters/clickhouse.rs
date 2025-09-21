use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use url::Url;

use crate::db::adapter::DbAdapter;
use crate::models::{Database, SchemaColumn, Table};

pub struct ClickHouseAdapter {
    client: Client,
    base_url: Url,
    username: Option<String>,
    password: Option<String>,
    database: Option<String>,
}

impl ClickHouseAdapter {
    pub async fn new(dsn: &str) -> Result<Self> {
        // dsn 示例: clickhouse://user:pass@host:8123/dbname
        let url = Url::parse(dsn)?;
        if url.scheme() != "clickhouse" {
            return Err(anyhow!("无效的 ClickHouse DSN"));
        }

        let username = url.username();
        let username = if username.is_empty() { None } else { Some(username.to_string()) };
        let password = url.password().map(|s| s.to_string());

        let database = url.path().trim_start_matches('/');
        let database = if database.is_empty() { None } else { Some(database.to_string()) };

        // 构建 HTTP 基础地址 http(s)://host:port
        let mut base_url = Url::parse(&format!(
            "http://{}:{}",
            url.host_str().ok_or_else(|| anyhow!("缺少主机"))?,
            url.port_or_known_default().unwrap_or(8123)
        ))?;

        // ClickHouse 默认用 HTTP 协议；若未来支持 TLS 可切换为 https
        base_url.set_scheme("http").ok();

        let client = Client::builder().build()?;

        Ok(Self { client, base_url, username, password, database })
    }

    async fn query_json(&self, sql: &str, database: Option<&str>) -> Result<Value> {
        let mut url = self.base_url.clone();
        url.set_path("/");
        let mut req = self.client.post(url).query(&[("query", format!("{} FORMAT JSON", sql))]);
        if let Some(db) = database.or(self.database.as_deref()) {
            req = req.query(&[("database", db.to_string())]);
        }
        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            req = req.basic_auth(u, Some(p));
        }
        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(anyhow!("ClickHouse 错误: {}", text));
        }
        let v: Value = serde_json::from_str(&text)?;
        Ok(v)
    }

    async fn exec(&self, sql: &str, database: Option<&str>) -> Result<u64> {
        let mut url = self.base_url.clone();
        url.set_path("/");
        let mut req = self.client.post(url).body(sql.to_string());
        if let Some(db) = database.or(self.database.as_deref()) {
            req = req.query(&[("database", db.to_string())]);
        }
        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            req = req.basic_auth(u, Some(p));
        }
        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(anyhow!("ClickHouse 错误: {}", text));
        }
        Ok(0)
    }
}

#[async_trait]
impl DbAdapter for ClickHouseAdapter {
    fn driver_name(&self) -> &'static str { "ClickHouse" }
    fn keywords(&self) -> &'static [&'static str] {
        &[
            "SELECT","FROM","WHERE","INSERT","INTO","VALUES","CREATE","TABLE","DROP","ALTER",
            "DESCRIBE","SHOW","DATABASES","TABLES","LIMIT","ORDER","BY","GROUP","FORMAT","JSON",
        ]
    }
    fn system_databases(&self) -> &'static [&'static str] { &["INFORMATION_SCHEMA", "system"] }
    fn supports_use_database(&self) -> bool { false }
    fn quote_ident(&self, ident: &str) -> String { format!("`{}`", ident.replace('`', "``")) }

    async fn get_databases(&self) -> Result<Vec<Database>> {
        let v = self.query_json("SHOW DATABASES", None).await?;
        let mut out = Vec::new();
        if let Some(rows) = v.get("data").and_then(|d| d.as_array()) {
            for row in rows {
                if let Some(name) = row.get("name").and_then(|s| s.as_str()) {
                    if ["system"].contains(&name) { continue; }
                    out.push(Database::with_details(name.to_string(), None, None, None));
                } else if let Some(name) = row.get("database").and_then(|s| s.as_str()) {
                    if ["system"].contains(&name) { continue; }
                    out.push(Database::with_details(name.to_string(), None, None, None));
                }
            }
        }
        Ok(out)
    }

    async fn get_tables(&self, database_name: &str) -> Result<Vec<Table>> {
        let sql = format!("SHOW TABLES FROM {}", self.quote_ident(database_name));
        let v = self.query_json(&sql, None).await?;
        let mut out = Vec::new();
        if let Some(rows) = v.get("data").and_then(|d| d.as_array()) {
            for row in rows {
                if let Some(name) = row.get("name").and_then(|s| s.as_str())
                    .or_else(|| row.get("table").and_then(|s| s.as_str())) {
                    out.push(Table::with_details(name.to_string(), None, None, None, None));
                }
            }
        }
        Ok(out)
    }

    async fn get_table_schema(&self, database_name: &str, table_name: &str) -> Result<(Vec<SchemaColumn>, Option<String>)> {
        let sql = format!(
            "DESCRIBE TABLE {}.{}",
            self.quote_ident(database_name),
            self.quote_ident(table_name)
        );
        let v = self.query_json(&sql, None).await?;
        let mut cols = Vec::new();
        if let Some(rows) = v.get("data").and_then(|d| d.as_array()) {
            for row in rows {
                let name = row.get("name").and_then(|s| s.as_str()).unwrap_or("").to_string();
                let data_type = row.get("type").and_then(|s| s.as_str()).unwrap_or("").to_string();
                let default_type = row.get("default_type").and_then(|s| s.as_str()).map(|s| s.to_string());
                let comment = row.get("comment").and_then(|s| s.as_str()).map(|s| s.to_string());
                cols.push(SchemaColumn::with_details(name, data_type, true, default_type, None, comment));
            }
        }
        Ok((cols, None))
    }

    async fn execute_query_raw(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let v = self.query_json(query, None).await?;
        let mut headers = Vec::new();
        let mut rows_out = Vec::new();
        if let Some(meta) = v.get("meta").and_then(|m| m.as_array()) {
            for col in meta {
                if let Some(n) = col.get("name").and_then(|s| s.as_str()) {
                    headers.push(n.to_string());
                }
            }
        }
        if let Some(rows) = v.get("data").and_then(|d| d.as_array()) {
            for row in rows {
                let mut one = Vec::new();
                for h in &headers {
                    let cell = row.get(h).cloned().unwrap_or(Value::Null);
                    one.push(match cell {
                        Value::Null => "NULL".to_string(),
                        Value::Bool(b) => if b {"1".to_string()} else {"0".to_string()},
                        Value::Number(n) => n.to_string(),
                        Value::String(s) => s,
                        other => other.to_string(),
                    });
                }
                rows_out.push(one);
            }
        }
        Ok((headers, rows_out))
    }

    async fn execute_non_query(&self, query: &str) -> Result<u64> {
        self.exec(query, None).await
    }

    async fn get_version(&self) -> Result<String> {
        let v = self.query_json("SELECT version() AS v", None).await?;
        if let Some(rows) = v.get("data").and_then(|d| d.as_array()) {
            if let Some(first) = rows.first() { return Ok(first.get("v").and_then(|s| s.as_str()).unwrap_or("").to_string()); }
        }
        Ok(String::new())
    }

    async fn get_current_user(&self) -> Result<String> {
        let v = self.query_json("SELECT currentUser() AS u", None).await?;
        if let Some(rows) = v.get("data").and_then(|d| d.as_array()) {
            if let Some(first) = rows.first() { return Ok(first.get("u").and_then(|s| s.as_str()).unwrap_or("").to_string()); }
        }
        Ok(String::new())
    }
}

