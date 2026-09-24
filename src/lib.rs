//! synh8 - A synaptic-inspired TUI for apt management
//!
//! The library holds everything that does not draw to the terminal; the
//! `synh8` binary adds the event loop, App state and rendering on top.

pub mod apt;
pub mod core;
pub mod keymap;
pub mod progress;
pub mod search;
pub mod types;
pub mod version;
