use thiserror::Error;

/// Core error types for the BerryBot platform.
#[derive(Error, Debug)]
pub enum BerryError {
    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Task error: {0}")]
    Task(String),

    #[error("Tool execution error in '{name}': {message}")]
    ToolExecution { name: String, message: String },

    #[error("Tool '{0}' not found")]
    ToolNotFound(String),

    #[error("Event bus error: {0}")]
    Bus(String),

    #[error("LLM provider error: {0}")]
    Llm(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Channel or collaboration error: {0}")]
    Collaboration(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type BerryResult<T> = Result<T, BerryError>;
