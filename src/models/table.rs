use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub comment: Option<String>,
    pub rows: Option<u64>,
    pub size: Option<u64>,
    pub engine: Option<String>,
}

impl Table {

    pub fn with_details(
        name: String,
        comment: Option<String>,
        rows: Option<u64>,
        size: Option<u64>,
        engine: Option<String>,
    ) -> Self {
        Self {
            name,
            comment,
            rows,
            size,
            engine,
        }
    }
}
