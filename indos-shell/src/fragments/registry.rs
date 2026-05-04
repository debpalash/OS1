//! Fragment component registry
//!
//! Maps component names to their implementations.
//! Follows the CopilotKit/Tambo pattern: pre-built components
//! registered with schemas so the LLM knows what it can render.

use std::collections::HashMap;

/// Registry of available fragment components
pub struct FragmentRegistry {
    components: HashMap<String, ComponentEntry>,
}

/// A registered component
pub struct ComponentEntry {
    /// Component name (e.g., "file-browser")
    pub name: String,

    /// Human-readable description for the LLM
    pub description: String,

    /// JSON schema describing the component's props
    pub props_schema: serde_json::Value,

    /// Whether this component is interactable (persistent)
    pub interactive: bool,

    /// Path to the webview HTML/JS for this component
    pub webview_path: String,
}

impl FragmentRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            components: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    /// Register all built-in fragment components
    fn register_builtins(&mut self) {
        self.register(ComponentEntry {
            name: "file-browser".into(),
            description: "Browse and manage files and directories".into(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path to browse" },
                    "view": { "type": "string", "enum": ["grid", "list", "tree"] },
                    "show_hidden": { "type": "boolean" }
                },
                "required": ["path"]
            }),
            interactive: true,
            webview_path: "fragments/file-browser/index.html".into(),
        });

        self.register(ComponentEntry {
            name: "terminal".into(),
            description: "Interactive terminal emulator".into(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "Working directory" },
                    "command": { "type": "string", "description": "Initial command to run" }
                }
            }),
            interactive: true,
            webview_path: "fragments/terminal/index.html".into(),
        });

        self.register(ComponentEntry {
            name: "code-editor".into(),
            description: "View and edit source code files".into(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path to open" },
                    "language": { "type": "string" },
                    "read_only": { "type": "boolean" }
                },
                "required": ["file"]
            }),
            interactive: true,
            webview_path: "fragments/code-editor/index.html".into(),
        });

        self.register(ComponentEntry {
            name: "system-monitor".into(),
            description: "Real-time system resource monitoring (CPU, RAM, GPU, disk, network)".into(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "view": { "type": "string", "enum": ["overview", "cpu", "memory", "gpu", "network"] }
                }
            }),
            interactive: true,
            webview_path: "fragments/system-monitor/index.html".into(),
        });

        self.register(ComponentEntry {
            name: "chart".into(),
            description: "Render data visualizations and charts".into(),
            props_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "type": { "type": "string", "enum": ["bar", "line", "pie", "scatter"] },
                    "data": { "type": "array" },
                    "title": { "type": "string" }
                },
                "required": ["type", "data"]
            }),
            interactive: false,
            webview_path: "fragments/chart/index.html".into(),
        });
    }

    pub fn register(&mut self, entry: ComponentEntry) {
        self.components.insert(entry.name.clone(), entry);
    }

    pub fn get(&self, name: &str) -> Option<&ComponentEntry> {
        self.components.get(name)
    }

    pub fn list(&self) -> Vec<&ComponentEntry> {
        self.components.values().collect()
    }

    /// Generate a schema document that can be sent to the LLM
    /// so it knows what fragments are available
    pub fn to_llm_schema(&self) -> serde_json::Value {
        let components: Vec<serde_json::Value> = self.components.values().map(|c| {
            serde_json::json!({
                "name": c.name,
                "description": c.description,
                "props": c.props_schema,
                "interactive": c.interactive
            })
        }).collect();

        serde_json::json!({
            "available_fragments": components
        })
    }
}
