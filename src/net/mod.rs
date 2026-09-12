pub mod relay;
pub mod wire;

pub use wire::{Hello, RoomCode};

#[cfg(feature = "game")]
mod client;
#[cfg(feature = "game")]
mod protocol;
#[cfg(feature = "game")]
mod session;

#[cfg(feature = "game")]
pub use protocol::Msg;
#[cfg(feature = "game")]
pub use session::*;
