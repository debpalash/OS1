//! Focus mode — immersive workspace

pub struct FocusMode {
    /// Primary fragment taking full screen
    pub primary_fragment: Option<String>,
}

impl FocusMode {
    pub fn new() -> Self {
        Self {
            primary_fragment: None,
        }
    }
}
