//! Fragment renderer — renders A2UI components in sandboxed webviews

/// Renders fragment components in isolated webview surfaces
pub struct FragmentRenderer {
    /// Active webview instances
    active_count: usize,
}

impl FragmentRenderer {
    pub fn new() -> Self {
        Self { active_count: 0 }
    }

    /// Render a fragment from an A2UI spec
    pub fn render(&mut self, _component: &str, _props: &serde_json::Value) {
        // TODO: Create sandboxed webview
        // TODO: Load component HTML from registry
        // TODO: Inject props as JSON
        // TODO: Mount webview as Wayland subsurface
        self.active_count += 1;
    }

    /// Destroy a fragment
    pub fn destroy(&mut self, _fragment_id: &str) {
        // TODO: Tear down webview
        self.active_count = self.active_count.saturating_sub(1);
    }

    pub fn active_count(&self) -> usize {
        self.active_count
    }
}
