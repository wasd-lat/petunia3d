//! Capability-limited Lua 5.4 plugin host (P1-03, `mlua`).
//!
//! Plugins receive Petunia API contracts, never raw `egui::Context`,
//! `wgpu::Device`, application state, or unrestricted filesystem. Every
//! script declares its [`Capabilities`]; the host enforces them before and
//! during execution:
//!
//! ```text
//! Lua source
//!     ↓ declare Capabilities
//! Host validates (allowlisted commands/queries, fs roots, network off)
//!     ↓
//! Sandboxed `mlua` state (no io/os/process libs unless granted)
//!     ↓
//! Validated CommandIntent / query DTOs out (execution stays with the app)
//! ```

pub mod dispatch;
pub mod host;

pub use dispatch::{answer_query, apply_intents};
pub use host::{
    CapabilityError, Host, HostConfig, LuaCommand, PluginCapabilities, PluginError, PluginId,
    ValidatedIntent,
};
