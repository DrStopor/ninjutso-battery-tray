//! Logical geometry. The frontend owns text measurement and DPI conversion.
pub const FLYOUT_WIDTH: i32 = 292;
pub const FLYOUT_HEIGHT: i32 = 140;
pub const FLYOUT_CORNER_DIAMETER: i32 = 20;

/// Shared vertical metric axis and a fixed power region.
/// Current charge, DPI and localized status text must not participate in layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargeRowLayout {
    pub dpi_left: f32,
    pub power_left: f32,
    pub power_width: f32,
}

impl ChargeRowLayout {
    pub const RATE_CENTER: f32 = 134.0;

    pub fn new(content_width: f32, dpi_width: f32) -> Self {
        let power_width = 104.0;
        let power_left = content_width - power_width;
        let dpi_left = (Self::RATE_CENTER - dpi_width / 2.0).round();
        Self {
            dpi_left,
            power_left,
            power_width,
        }
    }
}

/// Align to the charge's ink edge, preferring the percent midline as a lower
/// guide. Two readable lines take priority when that recommended height is tight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DpiBadgeLayout {
    pub top: f32,
    pub height: f32,
}

impl DpiBadgeLayout {
    pub fn between_guides(charge_top: f32, percent_midline: f32) -> Self {
        let top = charge_top.ceil();
        let height = (percent_midline - top).floor().clamp(32.0, 34.0);
        Self { top, height }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Bounds {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
    pub const fn width(self) -> i32 {
        self.right - self.left
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlyoutLayout {
    pub frame: Bounds,
    pub background: Bounds,
    pub top_edge: Bounds,
    pub title: Bounds,
    pub badge: Option<Bounds>,
    pub badge_indicator: Bounds,
    pub badge_text: Bounds,
    pub percent: Bounds,
    pub status_indicator: Bounds,
    pub primary: Bounds,
    pub secondary: Bounds,
    pub meter: Bounds,
    pub badge_corner_diameter: i32,
    pub meter_corner_diameter: i32,
}

impl FlyoutLayout {
    pub fn new(connection_text_width: Option<i32>) -> Self {
        let pill_left = connection_text_width
            .map(|width| 276 - width.saturating_add(36).clamp(58, 122))
            .unwrap_or(276);
        Self {
            frame: Bounds::new(0, 0, FLYOUT_WIDTH, FLYOUT_HEIGHT),
            background: Bounds::new(1, 1, FLYOUT_WIDTH - 1, FLYOUT_HEIGHT - 1),
            top_edge: Bounds::new(30, 1, FLYOUT_WIDTH - 30, 2),
            title: Bounds::new(16, 9, pill_left - 8, 35),
            badge: connection_text_width.map(|_| Bounds::new(pill_left, 10, 276, 34)),
            badge_indicator: Bounds::new(pill_left + 11, 20, pill_left + 15, 24),
            badge_text: Bounds::new(pill_left + 24, 10, 268, 34),
            percent: Bounds::new(16, 38, 136, 105),
            status_indicator: Bounds::new(145, 50, 148, 66),
            primary: Bounds::new(156, 42, 276, 72),
            secondary: Bounds::new(145, 72, 276, 108),
            meter: Bounds::new(16, 116, 276, 124),
            badge_corner_diameter: 14,
            meter_corner_diameter: 8,
        }
    }

    pub fn meter_fill(self, percent: Option<u8>) -> Option<Bounds> {
        let percent = percent?.min(100);
        if percent == 0 {
            return None;
        }
        let width = (self.meter.width() as u16 * u16::from(percent)).div_ceil(100) as i32;
        Some(Bounds::new(
            self.meter.left,
            self.meter.top,
            self.meter.left
                + width
                    .max(self.meter_corner_diameter)
                    .min(self.meter.width()),
            self.meter.bottom,
        ))
    }
}

pub fn popup_position(point: Point, work: Option<Bounds>) -> Point {
    let Some(work) = work else {
        return Point {
            x: (point.x - FLYOUT_WIDTH + 24).max(8),
            y: (point.y - FLYOUT_HEIGHT - 18).max(8),
        };
    };
    let margin = 8;
    let mut x = point.x - FLYOUT_WIDTH + 24;
    let mut y = point.y - FLYOUT_HEIGHT - 18;
    if point.y >= work.bottom {
        y = work.bottom - FLYOUT_HEIGHT - margin;
    } else if point.y < work.top {
        y = work.top + margin;
    } else if point.x >= work.right {
        x = work.right - FLYOUT_WIDTH - margin;
        y = point.y - FLYOUT_HEIGHT + 24;
    } else if point.x < work.left {
        x = work.left + margin;
        y = point.y - FLYOUT_HEIGHT + 24;
    }
    let min_x = work.left + margin;
    let max_x = (work.right - FLYOUT_WIDTH - margin).max(min_x);
    let min_y = work.top + margin;
    let max_y = (work.bottom - FLYOUT_HEIGHT - margin).max(min_y);
    Point {
        x: x.clamp(min_x, max_x),
        y: y.clamp(min_y, max_y),
    }
}
