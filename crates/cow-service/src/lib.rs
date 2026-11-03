//! Application services for the local CowAI runtime.
//!
//! This crate contains the local bootstrap flow and the channel/message
//! services that keep SQLite persistence and the event bus in sync.

mod assignments;
mod bootstrap;
mod channels;
mod crew_runner;
mod templates;

pub use assignments::AssignmentService;
pub use bootstrap::{bootstrap_local, bootstrap_local_named, LocalContext};
pub use channels::ChannelService;
pub use crew_runner::CrewRunner;
pub use templates::{builtin_templates, TemplateService};
