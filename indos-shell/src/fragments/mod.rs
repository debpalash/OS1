//! Fragment system — A2UI component registry and rendering
//!
//! Three tiers:
//!   Tier 1: Native iced widgets (this module) — <50ms render
//!   Tier 2: Tambo interactable (future) — schema-defined, webview
//!   Tier 3: OpenUI generative (future) — LLM → HTML/CSS, webview
//!
//! The orchestrator sends A2UI JSON:
//! ```json
//! { "type": "fragment", "fragment": {
//!     "component": "system-monitor",
//!     "props": { "refresh_interval": 2 }
//! }}
//! ```
//! The shell looks up "system-monitor" in the registry and renders it.

pub mod registry;
pub mod system_monitor;
pub mod text_block;
pub mod terminal;
pub mod file_list;
pub mod code_view;
pub mod chart;
pub mod markdown;
