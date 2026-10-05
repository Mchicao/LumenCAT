use gpui::*;
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    Light,
    #[default]
    Dark,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Accent {
    #[default]
    Blue,
    Teal,
    Violet,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    pub mode: ColorMode,
    pub accent: Accent,
}

impl Appearance {
    pub fn load(path: &Path) -> io::Result<Self> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(16_385).read_to_end(&mut bytes)?;
        if bytes.len() > 16_384 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Configuración de apariencia demasiado grande",
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    pub fn save(self, path: &Path) -> io::Result<()> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut temporary, &self)?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        temporary.persist(path).map_err(|error| error.error)?;
        Ok(())
    }
}

pub fn settings_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("LumenCAT/settings")
}

thread_local! {
    // ponytail: una paleta por hilo UI; temas independientes por ventana requieren estado de ventana.
    static APPEARANCE: Cell<Appearance> = const { Cell::new(Appearance { mode: ColorMode::Dark, accent: Accent::Blue }) };
}

pub struct Theme;

impl Theme {
    pub fn apply(appearance: Appearance) {
        APPEARANCE.set(appearance);
    }

    pub fn sync_components(cx: &mut App) {
        use gpui::component::{Theme as KitTheme, ThemeMode};
        let mode = APPEARANCE.with(|a| match a.get().mode {
            ColorMode::Light => ThemeMode::Light,
            ColorMode::Dark => ThemeMode::Dark,
        });
        KitTheme::change(mode, None, cx);
        KitTheme::update(cx, |theme| {
            theme.font_family = "Segoe UI".into();
            theme.colors = Self::component_colors();
        });
    }

    pub fn component_colors() -> gpui::component::ThemeColor {
        use gpui::component::ThemeColor;
        let defaults = APPEARANCE.with(|a| match a.get().mode {
            ColorMode::Light => ThemeColor::light(),
            ColorMode::Dark => ThemeColor::dark(),
        });
        ThemeColor {
            background: Self::bg_surface().into(),
            foreground: Self::text_primary().into(),
            border: Self::border_subtle().into(),
            input: Self::border_medium().into(),
            ring: Self::sky().into(),
            muted: Self::bg_subtle().into(),
            muted_foreground: Self::text_muted().into(),
            accent: Self::bg_hover().into(),
            accent_foreground: Self::text_primary().into(),
            primary: Self::sky().into(),
            primary_foreground: Self::text_on_accent().into(),
            primary_hover: Self::sky().into(),
            primary_active: Self::sky().into(),
            button_primary: Self::sky().into(),
            button_primary_foreground: Self::text_on_accent().into(),
            button_primary_hover: Self::sky().into(),
            button_primary_active: Self::sky().into(),
            button_secondary: Self::bg_subtle().into(),
            button_secondary_foreground: Self::text_primary().into(),
            button_secondary_hover: Self::bg_hover().into(),
            button_secondary_active: Self::bg_hover().into(),
            button_success: Self::emerald().into(),
            button_success_foreground: Self::text_on_accent().into(),
            button_success_hover: Self::emerald_hover().into(),
            button_success_active: Self::emerald_hover().into(),
            button_danger: Self::rose().into(),
            button_danger_foreground: Self::text_on_accent().into(),
            button_danger_hover: Self::rose().into(),
            button_danger_active: Self::rose().into(),
            popover: Self::bg_surface().into(),
            popover_foreground: Self::text_primary().into(),
            ..*defaults
        }
    }

    fn color(light: u32, dark: u32) -> Rgba {
        rgb(APPEARANCE.with(|a| {
            if a.get().mode == ColorMode::Light {
                light
            } else {
                dark
            }
        }))
    }

    fn tint(mut color: Rgba) -> Rgba {
        color.a = 0.14;
        color
    }

    pub fn bg_app() -> Rgba {
        Self::color(0xf3f5f8, 0x0d1117)
    }
    pub fn bg_sidebar() -> Rgba {
        Self::color(0xeef1f5, 0x11151c)
    }
    pub fn bg_surface() -> Rgba {
        Self::color(0xffffff, 0x161b22)
    }
    pub fn bg_card() -> Rgba {
        Self::color(0xffffff, 0x1b2028)
    }
    pub fn bg_subtle() -> Rgba {
        Self::color(0xe5eaf2, 0x21262d)
    }
    pub fn bg_hover() -> Rgba {
        Self::color(0xdae2ef, 0x282e38)
    }
    pub fn bg_selected() -> Rgba {
        Self::tint(Self::sky())
    }
    pub fn border_subtle() -> Rgba {
        Self::color(0xd2dae6, 0x30363d)
    }
    pub fn border_medium() -> Rgba {
        Self::color(0x9eaec3, 0x475569)
    }
    pub fn border_accent() -> Rgba {
        Self::sky()
    }
    pub fn border_selected() -> Rgba {
        Self::sky()
    }
    pub fn text_primary() -> Rgba {
        Self::color(0x182435, 0xf0f6fc)
    }
    pub fn text_secondary() -> Rgba {
        Self::color(0x475569, 0xb3becd)
    }
    pub fn text_muted() -> Rgba {
        Self::color(0x526174, 0x94a3b8)
    }
    pub fn text_accent() -> Rgba {
        Self::sky()
    }
    pub fn text_on_accent() -> Rgba {
        Self::color(0xffffff, 0x0a101d)
    }
    pub fn emerald() -> Rgba {
        Self::color(0x047857, 0x10b981)
    }
    pub fn emerald_hover() -> Rgba {
        Self::color(0x065f46, 0x34d399)
    }
    pub fn emerald_bg() -> Rgba {
        Self::tint(Self::emerald())
    }
    pub fn amber() -> Rgba {
        Self::color(0x92400e, 0xf59e0b)
    }
    pub fn amber_bg() -> Rgba {
        Self::tint(Self::amber())
    }
    pub fn rose() -> Rgba {
        Self::color(0xbe123c, 0xfb7185)
    }
    pub fn rose_bg() -> Rgba {
        Self::tint(Self::rose())
    }
    pub fn sky() -> Rgba {
        let (light, dark) = APPEARANCE.with(|a| match a.get().accent {
            Accent::Blue => (0x2459a9, 0x65b5fb),
            Accent::Teal => (0x0f766e, 0x2dd4bf),
            Accent::Violet => (0x6d28d9, 0xa78bfa),
        });
        Self::color(light, dark)
    }
    pub fn sky_bg() -> Rgba {
        Self::tint(Self::sky())
    }
    pub fn violet() -> Rgba {
        Self::color(0x6d28d9, 0xa78bfa)
    }
    pub fn violet_bg() -> Rgba {
        Self::tint(Self::violet())
    }
    pub fn slate() -> Rgba {
        Self::color(0x475569, 0x94a3b8)
    }
    pub fn slate_bg() -> Rgba {
        Self::tint(Self::slate())
    }
}
