mod adapter; // trait 与工厂
mod adapters; // 各后端适配器实现

pub use adapter::{DbAdapter, new_adapter};
