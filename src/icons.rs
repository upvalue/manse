/// Application icons for terminal panels
use eframe::egui;

/// Embedded icon images
const NEOVIM_ICON: &[u8] = include_bytes!("../assets/icons/neovim.png");

/// Known application icons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppIcon {
    Neovim,
}

impl AppIcon {
    /// Returns the image URI for use with egui
    pub fn image_uri(&self) -> &'static str {
        match self {
            AppIcon::Neovim => "bytes://neovim_icon",
        }
    }

    /// Returns the raw image bytes
    pub fn image_bytes(&self) -> &'static [u8] {
        match self {
            AppIcon::Neovim => NEOVIM_ICON,
        }
    }

    /// Load this icon into the egui context (safe to call multiple times)
    pub fn load(&self, ctx: &egui::Context) {
        // include_bytes is idempotent - safe to call every frame
        ctx.include_bytes(self.image_uri(), self.image_bytes());
    }
}

/// Detects the application icon from a terminal title
pub fn detect_icon(title: &str) -> Option<AppIcon> {
    let title_lower = title.to_lowercase();

    // Neovim detection
    if title_lower.contains("nvim") || title_lower.contains("neovim") {
        return Some(AppIcon::Neovim);
    }

    None
}
