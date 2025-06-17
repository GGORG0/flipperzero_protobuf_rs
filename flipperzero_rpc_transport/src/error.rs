// TODO: Use thiserror!
pub type Error = Box<dyn std::error::Error + Send + Sync>;