mod connection;
mod queries;
mod adapter; // 新的适配器模式

pub use adapter::{DbAdapter, new_adapter};
