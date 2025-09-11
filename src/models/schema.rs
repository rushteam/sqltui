use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaColumn {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub extra: Option<String>,
    pub comment: Option<String>,
}

impl SchemaColumn {

    pub fn with_details(
        name: String,
        data_type: String,
        is_nullable: bool,
        default_value: Option<String>,
        extra: Option<String>,
        comment: Option<String>,
    ) -> Self {
        Self {
            name,
            data_type,
            is_nullable,
            default_value,
            extra,
            comment,
        }
    }
}
