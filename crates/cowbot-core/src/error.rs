use thiserror::Error;

/// Core error types for the CowBot platform.
#[derive(Error, Debug)]
pub enum CowError {
    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Task error: {0}")]
    Task(String),

    #[error("Tool execution error in '{name}': {message}")]
    ToolExecution { name: String, message: String },

    #[error("Tool '{0}' not found")]
    ToolNotFound(String),

    #[error("LLM provider error: {0}")]
    Llm(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Workspace package error: {0}")]
    Package(String),
}

pub type CowResult<T> = Result<T, CowError>;
