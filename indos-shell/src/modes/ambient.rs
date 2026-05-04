//! Ambient mode — passive contextual dashboard

pub struct AmbientMode {
    /// Widgets currently displayed
    active_widgets: Vec<String>,
}

impl AmbientMode {
    pub fn new() -> Self {
        Self {
            active_widgets: vec![
                "clock".into(),
                "system-stats".into(),
                "recent-activity".into(),
            ],
        }
    }
}
