use gpui::*;

pub struct Theme;

impl Theme {
    // Backgrounds
    pub fn bg_app() -> Rgba {
        rgb(0x0d1117)
    }

    pub fn bg_sidebar() -> Rgba {
        rgb(0x11151c)
    }

    pub fn bg_surface() -> Rgba {
        rgb(0x161b22)
    }

    pub fn bg_card() -> Rgba {
        rgb(0x1b2028)
    }

    pub fn bg_subtle() -> Rgba {
        rgb(0x21262d)
    }

    pub fn bg_hover() -> Rgba {
        rgb(0x282e38)
    }

    pub fn bg_selected() -> Rgba {
        rgb(0x19273f)
    }

    // Borders
    pub fn border_subtle() -> Rgba {
        rgb(0x21262d)
    }

    pub fn border_medium() -> Rgba {
        rgb(0x30363d)
    }

    pub fn border_accent() -> Rgba {
        rgb(0x38bdf8)
    }

    pub fn border_selected() -> Rgba {
        rgb(0x0284c7)
    }

    // Texts
    pub fn text_primary() -> Rgba {
        rgb(0xf0f6fc)
    }

    pub fn text_secondary() -> Rgba {
        rgb(0x94a3b8)
    }

    pub fn text_muted() -> Rgba {
        rgb(0x64748b)
    }

    pub fn text_accent() -> Rgba {
        rgb(0x38bdf8)
    }

    // Status Colors
    pub fn emerald() -> Rgba {
        rgb(0x10b981)
    }

    pub fn emerald_bg() -> Rgba {
        rgba(0x10b98125)
    }

    pub fn amber() -> Rgba {
        rgb(0xf59e0b)
    }

    pub fn amber_bg() -> Rgba {
        rgba(0xf59e0b25)
    }

    pub fn rose() -> Rgba {
        rgb(0xf43f5e)
    }

    pub fn rose_bg() -> Rgba {
        rgba(0xf43f5e25)
    }

    pub fn sky() -> Rgba {
        rgb(0x38bdf8)
    }

    pub fn sky_bg() -> Rgba {
        rgba(0x38bdf825)
    }

    pub fn violet() -> Rgba {
        rgb(0xa855f7)
    }

    pub fn violet_bg() -> Rgba {
        rgba(0xa855f725)
    }

    pub fn slate() -> Rgba {
        rgb(0x64748b)
    }

    pub fn slate_bg() -> Rgba {
        rgba(0x64748b25)
    }
}
