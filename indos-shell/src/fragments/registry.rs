//! Fragment registry — maps component names to render functions
//!
//! When the orchestrator sends an A2UI fragment, the shell looks up
//! the component name here and renders the matching native widget.

use iced::Element;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::chart;
use super::code_view;
use super::file_list;
use super::markdown;
use super::system_monitor;
use super::terminal;
use super::text_block;

/// A2UI fragment descriptor from the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentDescriptor {
    /// Component name (e.g. "system-monitor", "text-block", "terminal")
    pub component: String,

    /// Props passed to the component
    #[serde(default)]
    pub props: serde_json::Value,

    /// Unique fragment ID
    #[serde(default = "default_id")]
    pub id: String,
}

fn default_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Fragment registry — holds all registered Tier 1 native components
pub struct FragmentRegistry {
    components: HashMap<String, FragmentType>,
}

/// The types of native fragments we support
#[derive(Debug, Clone)]
pub enum FragmentType {
    TextBlock,
    SystemMonitor,
    Terminal,
    FileList,
    CodeView,
    Chart,
    Markdown,
}

impl FragmentRegistry {
    pub fn new() -> Self {
        let mut components = HashMap::new();
        components.insert("text-block".into(), FragmentType::TextBlock);
        components.insert("text".into(), FragmentType::TextBlock);
        components.insert("system-monitor".into(), FragmentType::SystemMonitor);
        components.insert("sysmon".into(), FragmentType::SystemMonitor);
        components.insert("terminal".into(), FragmentType::Terminal);
        components.insert("term".into(), FragmentType::Terminal);
        components.insert("file-list".into(), FragmentType::FileList);
        components.insert("files".into(), FragmentType::FileList);
        components.insert("code-view".into(), FragmentType::CodeView);
        components.insert("code".into(), FragmentType::CodeView);
        components.insert("chart".into(), FragmentType::Chart);
        components.insert("markdown".into(), FragmentType::Markdown);
        components.insert("md".into(), FragmentType::Markdown);

        Self { components }
    }

    /// Look up a component by name
    pub fn get(&self, name: &str) -> Option<&FragmentType> {
        self.components.get(name)
    }

    /// List all registered component names
    #[allow(dead_code)] // public API, not yet wired into the shell
    pub fn list(&self) -> Vec<&str> {
        self.components.keys().map(|s| s.as_str()).collect()
    }

    /// Render a fragment descriptor into an iced Element
    pub fn render<'a, M: 'a + Clone>(&self, descriptor: &FragmentDescriptor) -> Element<'a, M> {
        match self.get(&descriptor.component) {
            Some(FragmentType::TextBlock) => text_block::render(&descriptor.props),
            Some(FragmentType::SystemMonitor) => system_monitor::render(&descriptor.props),
            Some(FragmentType::Terminal) => terminal::render(&descriptor.props),
            Some(FragmentType::FileList) => file_list::render(&descriptor.props),
            Some(FragmentType::CodeView) => code_view::render(&descriptor.props),
            Some(FragmentType::Chart) => chart::render(&descriptor.props),
            Some(FragmentType::Markdown) => markdown::render(&descriptor.props),
            None => {
                // Unknown fragment — render a placeholder
                let msg = format!("⚠ Unknown fragment: {}", descriptor.component);
                iced::widget::container(
                    iced::widget::text(msg)
                        .size(14)
                        .color(iced::Color::from_rgb(0.9, 0.6, 0.2)),
                )
                .padding(8)
                .into()
            }
        }
    }
}
