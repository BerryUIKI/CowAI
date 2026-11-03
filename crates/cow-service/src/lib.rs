//! Application services for the local CowAI runtime.
//!
//! This crate contains the local bootstrap flow and the channel/message
//! services that keep SQLite persistence and the event bus in sync.

mod bootstrap;
mod channels;

pub use bootstrap::{bootstrap_local, bootstrap_local_named, LocalContext};
pub use channels::ChannelService;
