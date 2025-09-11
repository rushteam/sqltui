use anyhow::Result;
use sqlx::{MySql, Pool};
use tracing::info;

pub struct DatabaseConnection {
    pool: Pool<MySql>,
}

impl DatabaseConnection {
    pub async fn new(dsn: &str) -> Result<Self> {
        let pool = sqlx::MySqlPool::connect(dsn).await?;
        
        // 设置字符集
        sqlx::query("SET NAMES utf8mb4 COLLATE utf8mb4_unicode_ci")
            .execute(&pool)
            .await?;
        sqlx::query("SET character_set_client=utf8mb4")
            .execute(&pool)
            .await?;
        sqlx::query("SET character_set_connection=utf8mb4")
            .execute(&pool)
            .await?;
        sqlx::query("SET character_set_results=utf8mb4")
            .execute(&pool)
            .await?;
        
        info!("Connected to MySQL database");
        Ok(Self { pool })
    }

    pub fn get_pool(&self) -> &Pool<MySql> {
        &self.pool
    }

}
