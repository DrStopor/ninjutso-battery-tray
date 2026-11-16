use super::view::StatusTone;
use crate::settings::ThemePreference;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FontSpec {
    pub family: &'static str,
    pub size: i32,
    pub weight: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Typography {
    pub title: FontSpec,
    pub percent: FontSpec,
    pub primary: FontSpec,
    pub secondary: FontSpec,
    pub badge: FontSpec,
}

impl Typography {
    pub const DEFAULT: Self = Self {
        title: FontSpec {
            family: "Segoe UI Variable Text",
            size: 17,
            weight: 700,
        },
        percent: FontSpec {
            family: "Segoe UI Variable Display",
            size: 45,
            weight: 700,
        },
        primary: FontSpec {
            family: "Segoe UI Variable Text",
            size: 16,
            weight: 700,
        },
        secondary: FontSpec {
            family: "Segoe UI Variable Text",
            size: 12,
            weight: 400,
        },
        badge: FontSpec {
            family: "Segoe UI Variable Text",
            size: 12,
            weight: 400,
        },
    };
}

#[derive(Clone, Copy, Debug)]
pub struct FlyoutPalette {
    pub background: Rgb,
    pub border: Rgb,
    pub top_edge: Rgb,
    pub foreground: Rgb,
    pub muted: Rgb,
    pub badge: Rgb,
    pub meter_track: Rgb,
    pub healthy: Rgb,
    pub warning: Rgb,
    pub critical: Rgb,
    pub charging: Rgb,
    pub reading: Rgb,
    pub unavailable: Rgb,
}

impl FlyoutPalette {
    pub const fn accent(self, tone: StatusTone) -> Rgb {
        match tone {
            StatusTone::Healthy => self.healthy,
            StatusTone::Warning => self.warning,
            StatusTone::Critical | StatusTone::Error => self.critical,
            StatusTone::Charging => self.charging,
            StatusTone::Reading => self.reading,
            StatusTone::Unavailable => self.unavailable,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TrayPalette {
    pub healthy: Rgba,
    pub warning: Rgba,
    pub critical: Rgba,
    pub reading: Rgba,
    pub unavailable: Rgba,
    pub empty: Rgba,
    pub unavailable_empty: Rgba,
    pub outline: Rgba,
    pub foreground: Rgba,
    pub numeric_surface: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub flyout: FlyoutPalette,
    pub tray: TrayPalette,
    pub typography: Typography,
}

/// Shared by native and GPUI menu adapters, without platform color types.
#[derive(Clone, Copy, Debug)]
pub struct MenuPalette {
    pub background: Rgb,
    pub foreground: Rgb,
    pub muted: Rgb,
    pub highlight: Rgb,
    pub border: Rgb,
}

impl Theme {
    pub const fn resolve(preference: ThemePreference, system_is_dark: bool) -> Self {
        match preference {
            ThemePreference::Auto if system_is_dark => Self::FLUENT_DARK,
            ThemePreference::Auto | ThemePreference::Light => Self::LIGHT,
            ThemePreference::Dark => Self::FLUENT_DARK,
            ThemePreference::NinjaForce => Self::NINJAFORCE,
        }
    }

    pub const fn menu(self) -> MenuPalette {
        MenuPalette {
            background: self.flyout.background,
            foreground: self.flyout.foreground,
            muted: self.flyout.muted,
            highlight: self.flyout.badge,
            border: self.flyout.border,
        }
    }

    /// GPUI surfaces follow the OS appearance; tray pixels remain a separate palette.
    pub const FLUENT_DARK: Self = Self {
        flyout: FlyoutPalette {
            background: Rgb::new(25, 28, 33),
            border: Rgb::new(57, 62, 70),
            top_edge: Rgb::new(72, 77, 85),
            foreground: Rgb::new(241, 243, 247),
            muted: Rgb::new(165, 174, 188),
            badge: Rgb::new(40, 44, 52),
            meter_track: Rgb::new(53, 59, 69),
            healthy: Rgb::new(118, 218, 171),
            warning: Rgb::new(245, 199, 117),
            critical: Rgb::new(255, 144, 151),
            charging: Rgb::new(118, 218, 171),
            reading: Rgb::new(149, 189, 255),
            unavailable: Rgb::new(165, 174, 188),
        },
        ..Self::DARK
    };

    /// NinjaForce 1.0.8 uses #1E22AA for selected text and controls, with lilac
    /// translucent panels. Lift the surface luminance for readable small labels
    /// and flatten overlays into shared tokens rather than copying assets.
    /// Tray pixels stay independent of the card theme for taskbar readability.
    pub const NINJAFORCE: Self = Self {
        flyout: FlyoutPalette {
            background: Rgb::new(195, 189, 219),
            border: Rgb::new(155, 148, 196),
            top_edge: Rgb::new(231, 226, 241),
            foreground: Rgb::new(30, 34, 170),
            muted: Rgb::new(72, 65, 120),
            badge: Rgb::new(216, 210, 231),
            meter_track: Rgb::new(170, 161, 202),
            healthy: Rgb::new(30, 34, 170),
            warning: Rgb::new(134, 83, 0),
            critical: Rgb::new(168, 47, 70),
            charging: Rgb::new(24, 97, 71),
            reading: Rgb::new(59, 79, 149),
            unavailable: Rgb::new(105, 96, 129),
        },
        ..Self::DARK
    };

    /// Preserve the shipped appearance while extracting presentation from Win32.
    pub const DARK: Self = Self {
        flyout: FlyoutPalette {
            background: Rgb::new(16, 18, 22),
            border: Rgb::new(47, 53, 62),
            top_edge: Rgb::new(69, 76, 88),
            foreground: Rgb::new(247, 249, 252),
            muted: Rgb::new(143, 153, 166),
            badge: Rgb::new(28, 32, 38),
            meter_track: Rgb::new(35, 40, 47),
            healthy: Rgb::new(82, 211, 132),
            warning: Rgb::new(246, 190, 72),
            critical: Rgb::new(255, 91, 98),
            charging: Rgb::new(65, 225, 143),
            reading: Rgb::new(91, 156, 255),
            unavailable: Rgb::new(112, 123, 137),
        },
        tray: TrayPalette {
            healthy: Rgba::opaque(35, 190, 85),
            warning: Rgba::opaque(238, 180, 38),
            critical: Rgba::opaque(220, 55, 55),
            reading: Rgba::opaque(100, 145, 205),
            unavailable: Rgba::opaque(120, 120, 120),
            empty: Rgba::opaque(38, 43, 47),
            unavailable_empty: Rgba::opaque(42, 48, 52),
            outline: Rgba::opaque(0, 0, 0),
            foreground: Rgba::opaque(255, 255, 255),
            numeric_surface: Rgba::opaque(19, 23, 27),
        },
        typography: Typography::DEFAULT,
    };

    /// Light surface for theme-aware frontends. The Win32 fallback uses DARK.
    pub const LIGHT: Self = Self {
        flyout: FlyoutPalette {
            background: Rgb::new(250, 251, 253),
            border: Rgb::new(213, 219, 227),
            top_edge: Rgb::new(255, 255, 255),
            foreground: Rgb::new(24, 29, 36),
            muted: Rgb::new(83, 94, 109),
            badge: Rgb::new(233, 237, 243),
            meter_track: Rgb::new(224, 230, 237),
            healthy: Rgb::new(19, 128, 78),
            warning: Rgb::new(155, 97, 0),
            critical: Rgb::new(193, 45, 60),
            charging: Rgb::new(0, 126, 82),
            reading: Rgb::new(37, 102, 190),
            unavailable: Rgb::new(103, 115, 132),
        },
        tray: TrayPalette {
            healthy: Rgba::opaque(19, 128, 78),
            warning: Rgba::opaque(155, 97, 0),
            critical: Rgba::opaque(193, 45, 60),
            reading: Rgba::opaque(37, 102, 190),
            unavailable: Rgba::opaque(103, 115, 132),
            empty: Rgba::opaque(226, 231, 238),
            unavailable_empty: Rgba::opaque(226, 231, 238),
            outline: Rgba::opaque(255, 255, 255),
            foreground: Rgba::opaque(24, 29, 36),
            numeric_surface: Rgba::opaque(250, 251, 253),
        },
        typography: Typography::DEFAULT,
    };
}
