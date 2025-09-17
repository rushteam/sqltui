use anyhow::Result;
use sqlx::{MySqlPool, Row, Column};
use crate::models::{Database, Table, SchemaColumn};

pub struct DatabaseQueries {
    pool: MySqlPool,
}

impl DatabaseQueries {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn get_databases(&self) -> Result<Vec<Database>> {
        // 直接使用 SHOW DATABASES（更兼容）
        self.get_databases_simple().await
    }
    
    async fn try_get_databases_detailed(&self) -> Result<Vec<Database>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                SCHEMA_NAME as name,
                DEFAULT_CHARACTER_SET_NAME as charset,
                DEFAULT_COLLATION_NAME as collation,
                (SELECT COUNT(*) FROM information_schema.TABLES WHERE TABLE_SCHEMA = SCHEMATA.SCHEMA_NAME) as table_count
            FROM information_schema.SCHEMATA 
            WHERE SCHEMA_NAME NOT IN ('information_schema', 'performance_schema', 'mysql', 'sys')
            ORDER BY SCHEMA_NAME
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let databases = rows
            .into_iter()
            .map(|row| {
                Database::with_details(
                    String::from_utf8_lossy(&row.get::<Vec<u8>, _>("name")).to_string(),
                    row.get::<Option<String>, _>("charset"),
                    row.get::<Option<String>, _>("collation"),
                    Some(row.get::<i64, _>("table_count") as u64),
                )
            })
            .collect();

        Ok(databases)
    }
    
    async fn get_databases_simple(&self) -> Result<Vec<Database>> {
        // 使用 SHOW DATABASES 作为备选方案
        let rows = sqlx::query("SHOW DATABASES")
            .fetch_all(&self.pool)
            .await?;
            
        let mut databases = Vec::new();
        
        for row in rows {
            // 安全地获取数据库名称，处理不同的数据类型
            let db_name = self.get_cell_value_as_string(&row, 0)?;
            
            // 跳过系统数据库
            if db_name == "information_schema" || 
               db_name == "performance_schema" || 
               db_name == "mysql" || 
               db_name == "sys" {
                continue;
            }
            
            // 尝试获取表数量（可能失败，但不影响基本功能）
            let table_count = self.get_table_count_simple(&db_name).await.ok();
            
            databases.push(Database::with_details(
                db_name,
                None, // charset
                None, // collation  
                table_count,
            ));
        }
        
        Ok(databases)
    }
    
    async fn get_table_count_simple(&self, database_name: &str) -> Result<u64> {
        let query = format!("SHOW TABLES FROM `{}`", database_name);
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.len() as u64)
    }

    pub async fn get_tables(&self, database_name: &str) -> Result<Vec<Table>> {
        // 直接使用 SHOW TABLES（更兼容）
        self.get_tables_simple(database_name).await
    }
    
    async fn try_get_tables_detailed(&self, database_name: &str) -> Result<Vec<Table>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                TABLE_NAME as name,
                TABLE_COMMENT as comment,
                TABLE_ROWS as `rows`,
                ROUND(((DATA_LENGTH + INDEX_LENGTH) / 1024 / 1024), 2) as size,
                ENGINE as engine
            FROM information_schema.TABLES 
            WHERE TABLE_SCHEMA = ? AND TABLE_TYPE = 'BASE TABLE'
            ORDER BY TABLE_NAME
            "#
        )
        .bind(database_name)
        .fetch_all(&self.pool)
        .await?;

        let tables = rows
            .into_iter()
            .map(|row| {
                let comment: String = String::from_utf8_lossy(&row.get::<Vec<u8>, _>("comment")).to_string();
                let rows: Option<i64> = row.get("rows");
                let size: Option<f64> = row.get("size");
                let engine: String = String::from_utf8_lossy(&row.get::<Vec<u8>, _>("engine")).to_string();
                
                Table::with_details(
                    String::from_utf8_lossy(&row.get::<Vec<u8>, _>("name")).to_string(),
                    if comment.is_empty() { None } else { Some(comment) },
                    rows.map(|r| r as u64),
                    size.map(|s| s as u64),
                    Some(engine),
                )
            })
            .collect();

        Ok(tables)
    }
    
    async fn get_tables_simple(&self, database_name: &str) -> Result<Vec<Table>> {
        // 使用 SHOW TABLES 作为备选方案
        let query = format!("SHOW TABLES FROM `{}`", database_name);
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await?;
            
        let mut tables = Vec::new();
        
        for row in rows {
            // 尝试直接获取 String，如果失败再使用 Vec<u8>
            let table_name = match row.try_get::<String, _>(0) {
                Ok(name) => name,
                Err(_) => {
                    // 如果 String 失败，尝试 Vec<u8>
                    String::from_utf8_lossy(&row.get::<Vec<u8>, _>(0)).to_string()
                }
            };
            
            tables.push(Table::with_details(
                table_name,
                None, // comment
                None, // rows
                None, // size
                None, // engine
            ));
        }
        
        Ok(tables)
    }

    pub async fn get_table_schema(&self, database_name: &str, table_name: &str) -> Result<(Vec<SchemaColumn>, Option<String>)> {
        // 首先尝试使用 information_schema（更详细的信息）
        let result = self.try_get_table_schema_detailed(database_name, table_name).await;
        
        if result.is_ok() {
            return result;
        }
        
        // 如果失败，回退到 SHOW COLUMNS（更兼容）
        self.get_table_schema_simple(database_name, table_name).await
    }
    
    async fn try_get_table_schema_detailed(&self, database_name: &str, table_name: &str) -> Result<(Vec<SchemaColumn>, Option<String>)> {
        // 获取表注释
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

        // 获取列信息
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

        let columns = rows
            .into_iter()
            .map(|row| {
                let is_nullable: String = String::from_utf8_lossy(&row.get::<Vec<u8>, _>("is_nullable")).to_string();
                let default_value: Option<String> = match row.try_get::<String, _>("default_value") {
                    Ok(val) => Some(val),
                    Err(_) => {
                        // 如果 String 失败，尝试 Vec<u8>
                        match row.try_get::<Vec<u8>, _>("default_value") {
                            Ok(bytes) => {
                                let decoded = String::from_utf8_lossy(&bytes).to_string();
                                if decoded.is_empty() { None } else { Some(decoded) }
                            },
                            Err(_) => None,
                        }
                    }
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
            })
            .collect();

        Ok((columns, table_comment))
    }
    
    async fn get_table_schema_simple(&self, database_name: &str, table_name: &str) -> Result<(Vec<SchemaColumn>, Option<String>)> {
        // 使用 SHOW COLUMNS 作为备选方案
        let query = format!("SHOW COLUMNS FROM `{}`.`{}`", database_name, table_name);
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await?;
            
        let mut columns = Vec::new();
        
        for row in rows {
            let field = String::from_utf8_lossy(&row.get::<Vec<u8>, _>(0)).to_string();
            let type_info = String::from_utf8_lossy(&row.get::<Vec<u8>, _>(1)).to_string();
            let null_info = String::from_utf8_lossy(&row.get::<Vec<u8>, _>(2)).to_string();
            let default_value: Option<String> = row.get(4);
            let extra = String::from_utf8_lossy(&row.get::<Vec<u8>, _>(5)).to_string();
            let comment = String::from_utf8_lossy(&row.get::<Vec<u8>, _>(6)).to_string();
            
            // 解析数据类型（简化处理）
            let data_type = if let Some(space_pos) = type_info.find(' ') {
                type_info[..space_pos].to_string()
            } else {
                type_info
            };
            
            columns.push(SchemaColumn::with_details(
                field,
                data_type,
                null_info == "YES",
                default_value,
                if extra.is_empty() { None } else { Some(extra) },
                if comment.is_empty() { None } else { Some(comment) },
            ));
        }
        
        Ok((columns, None)) // 简单模式下无法获取表注释
    }

    pub async fn get_mysql_version(&self) -> Result<String> {
        let row = sqlx::query("SELECT VERSION() as version")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<String, _>("version"))
    }

    pub async fn get_current_user(&self) -> Result<String> {
        let row = sqlx::query("SELECT USER() as user")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<String, _>("user"))
    }

    pub async fn execute_query(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::new();
        for row in rows {
            let mut map = serde_json::Map::new();
            for (i, column) in row.columns().iter().enumerate() {
                let value: serde_json::Value = row.try_get(i)?;
                map.insert(column.name().to_string(), value);
            }
            results.push(serde_json::Value::Object(map));
        }

        Ok(results)
    }

    pub async fn execute_use_command(&self, database_name: &str) -> Result<()> {
        // USE 命令不能使用预编译语句，使用 fetch_optional 来执行
        let sql = format!("USE `{}`", database_name);
        let _: Option<String> = sqlx::query_scalar(&sql)
            .fetch_optional(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn execute_query_raw(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;

        if rows.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        // 获取列名
        let headers: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect();

        // 获取数据行
        let mut data_rows = Vec::new();
        for row in rows {
            let mut row_data = Vec::new();
            for i in 0..row.columns().len() {
                let value = self.get_cell_value_as_string(&row, i)?;
                row_data.push(value);
            }
            data_rows.push(row_data);
        }

        Ok((headers, data_rows))
    }

    fn get_cell_value_as_string(&self, row: &sqlx::mysql::MySqlRow, index: usize) -> Result<String> {
        use sqlx::Row;
        
        // 尝试不同的数据类型
        if let Ok(value) = row.try_get::<String, _>(index) {
            return Ok(value);
        }
        
        if let Ok(value) = row.try_get::<i64, _>(index) {
            return Ok(value.to_string());
        }
        
        if let Ok(value) = row.try_get::<f64, _>(index) {
            return Ok(value.to_string());
        }
        
        if let Ok(value) = row.try_get::<bool, _>(index) {
            return Ok(if value { "1".to_string() } else { "0".to_string() });
        }
        
        if let Ok(value) = row.try_get::<chrono::NaiveDateTime, _>(index) {
            return Ok(value.format("%Y-%m-%d %H:%M:%S").to_string());
        }
        
        if let Ok(value) = row.try_get::<chrono::NaiveDate, _>(index) {
            return Ok(value.format("%Y-%m-%d").to_string());
        }
        
        if let Ok(value) = row.try_get::<chrono::NaiveTime, _>(index) {
            return Ok(value.format("%H:%M:%S").to_string());
        }
        
        if let Ok(value) = row.try_get::<Vec<u8>, _>(index) {
            // 尝试将Vec<u8>转换为字符串
            match String::from_utf8(value.clone()) {
                Ok(s) => return Ok(s),
                Err(_) => {
                    // 如果UTF-8转换失败，使用lossy转换
                    return Ok(String::from_utf8_lossy(&value).to_string());
                }
            }
        }
        
        if let Ok(value) = row.try_get::<serde_json::Value, _>(index) {
            return Ok(value.to_string());
        }
        
        // 如果所有类型都失败，返回NULL
        Ok("NULL".to_string())
    }
}
