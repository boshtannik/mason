pub mod event;
pub mod message;
pub mod permission;
pub mod session;
pub mod tool;

pub use event::GlobalEvent;
pub use message::Message;
pub use permission::Permission;
pub use session::Session;
pub use tool::ToolState;

/// Мета-сущность «событие» — отдельно, чтобы не конфликтовал с event::Event.
pub use event::Event as ServerEvent;