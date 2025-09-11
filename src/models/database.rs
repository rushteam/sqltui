use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub name: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub table_count: Option<u64>,
}

impl Database {

    pub fn with_details(name: String, charset: Option<String>, collation: Option<String>, table_count: Option<u64>) -> Self {
        Self {
            name,
            charset,
            collation,
            table_count,
        }
    }
}
