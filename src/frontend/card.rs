//! Declarative GPUI view. No HID, persistence, shell handles, or polling here.
use async_channel::Sender;
use gpui::{prelude::*, *};

use crate::application::{AppEvent, DismissReason};
use crate::i18n::Language;
use crate::model::{AppModel, BatteryStatus, ConnectionKind};
use crate::presentation::layout::{ChargeRowLayout, DpiBadgeLayout};
use crate::presentation::theme::{Rgb as Color, Theme};
use crate::presentation::view::{StatusTone, popup_view, status_text};

pub(super) const WIDTH: f32 = 304.0;
pub(super) const HEIGHT: f32 = 156.0;
const CHARGE_ROW_HEIGHT: f32 = 56.0;
const CHARGE_FONT_SIZE: f32 = 44.0;
const CHARGE_LINE_HEIGHT: f32 = 52.0;
const PERCENT_FONT_SIZE: f32 = 20.0;
const DPI_LABEL_SIZE: f32 = 20.0;
const DPI_VALUE_SIZE: f32 = 11.0;
const DPI_HORIZONTAL_PADDING: f32 = 1.0;
const METRIC_FONT_FAMILY: &str = "Segoe UI";

pub(super) struct BatteryCard {
    model: AppModel,
    generation: u64,
    events: Sender<AppEvent>,
    focus: FocusHandle,
    close_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl BatteryCard {
    pub(super) fn new(
        model: AppModel,
        generation: u64,
        events: Sender<AppEvent>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscriptions = vec![
            cx.observe_window_activation(window, |card, window, _| {
                if !window.is_window_active() {
                    card.dismiss(DismissReason::FocusLoss);
                }
            }),
            cx.observe_window_appearance(window, |_, _, cx| cx.notify()),
        ];
        let focus = cx.focus_handle();
        window.focus(&focus);
        let close_events = events.clone();
        window.on_window_should_close(cx, move |_, _| {
            let _ = close_events.try_send(AppEvent::Dismiss {
                generation,
                reason: DismissReason::CloseButton,
            });
            false
        });
        window.set_window_title(&status_text(&model.status, model.settings.language));
        Self {
            model,
            generation,
            events,
            focus,
            close_focus: cx.focus_handle().tab_stop(true),
            _subscriptions: subscriptions,
        }
    }

    pub(super) fn reopen(
        &mut self,
        model: AppModel,
        generation: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.generation = generation;
        let close_events = self.events.clone();
        window.on_window_should_close(cx, move |_, _| {
            let _ = close_events.try_send(AppEvent::Dismiss {
                generation,
                reason: DismissReason::CloseButton,
            });
            false
        });
        self.set_model(model, window, cx);
        window.focus(&self.focus);
        cx.notify();
    }

    pub(super) fn set_model(
        &mut self,
        model: AppModel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.model != model {
            window.set_window_title(&status_text(&model.status, model.settings.language));
            self.model = model;
            cx.notify();
        }
    }

    fn dismiss(&self, reason: DismissReason) {
        let _ = self.events.try_send(AppEvent::Dismiss {
            generation: self.generation,
            reason,
        });
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => self.dismiss(DismissReason::Escape),
            "tab" => {
                if event.keystroke.modifiers.shift {
                    window.focus_prev();
                } else {
                    window.focus_next();
                }
            }
            "enter" | "space" if self.close_focus.is_focused(window) => {
                self.dismiss(DismissReason::CloseButton)
            }
            _ => return,
        }
        cx.stop_propagation();
    }
}

fn color(value: Color) -> gpui::Rgba {
    rgb((u32::from(value.r) << 16) | (u32::from(value.g) << 8) | u32::from(value.b))
}

struct TextInk {
    advance: Pixels,
    left: Pixels,
    right: Pixels,
    ascent: Pixels,
}

/// Measure visible glyphs, not just font-wide cap-height/advance estimates.
fn measure_ink(
    system: &WindowTextSystem,
    font: FontId,
    size: Pixels,
    text: &str,
) -> Option<TextInk> {
    let mut advance = px(0.0);
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut ascent = 0.0_f32;
    for character in text.chars() {
        let bounds = system.typographic_bounds(font, size, character).ok()?;
        left = left.min(f32::from(advance + bounds.origin.x));
        right = right.max(f32::from(advance + bounds.origin.x + bounds.size.width));
        ascent = ascent.max(f32::from(bounds.origin.y + bounds.size.height));
        advance += system.advance(font, size, character).ok()?.width;
    }
    left.is_finite().then_some(TextInk {
        advance,
        left: px(left),
        right: px(right),
        ascent: px(ascent),
    })
}

impl Render for BatteryCard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Theme::resolve(
            self.model.settings.theme,
            matches!(
                window.appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ),
        )
        .flyout;
        let view = popup_view(&self.model.status, self.model.settings.language);
        let accent = color(palette.accent(view.tone));
        let foreground = color(palette.foreground);
        let muted = color(palette.muted);
        let badge = color(palette.badge);
        let font_family = if self.model.settings.language == Language::ChineseSimplified {
            "Microsoft YaHei UI"
        } else {
            "Segoe UI"
        };
        let close_label = self.model.settings.language.strings().close;
        let polling_rate_hint = self.model.settings.language.strings().polling_rate_hint;
        let dpi_hint = self.model.settings.language.strings().dpi_hint;
        let text_system = window.text_system();
        let charge_font = text_system.resolve_font(&Font {
            weight: FontWeight::SEMIBOLD,
            ..font(METRIC_FONT_FAMILY)
        });
        let percent_font = text_system.resolve_font(&font(METRIC_FONT_FAMILY));
        // GPUI 0.2.2 exposes a signed font descent on Windows, while its
        // painted line layout uses a positive descent. Match the latter.
        let ascent = text_system.ascent(charge_font, px(CHARGE_FONT_SIZE));
        let descent = px(f32::from(text_system.descent(charge_font, px(CHARGE_FONT_SIZE))).abs());
        let baseline = px((CHARGE_ROW_HEIGHT - CHARGE_LINE_HEIGHT) / 2.0)
            + (px(CHARGE_LINE_HEIGHT) - ascent - descent) / 2.0
            + ascent;
        let charge_ink = measure_ink(text_system, charge_font, px(CHARGE_FONT_SIZE), "88");
        let charge_top = baseline
            - charge_ink.map_or_else(
                || text_system.cap_height(charge_font, px(CHARGE_FONT_SIZE)),
                |ink| ink.ascent,
            );
        let percent_midline =
            baseline - text_system.cap_height(percent_font, px(PERCENT_FONT_SIZE)) / 2.0;
        let dpi_layout = DpiBadgeLayout::between_guides(charge_top.into(), percent_midline.into());
        let label_ink =
            measure_ink(text_system, charge_font, px(DPI_LABEL_SIZE), "DPI").unwrap_or(TextInk {
                advance: px(28.0),
                left: px(1.0),
                right: px(27.0),
                ascent: px(14.0),
            });
        let label_width = f32::from(label_ink.advance).ceil();
        let value_left = f32::from(label_ink.left).ceil();
        let value_width = (f32::from(label_ink.right).floor() - value_left).max(1.0);
        let dpi_badge_width = label_width + 2.0 * (DPI_HORIZONTAL_PADDING + 1.0);
        let value_advance = measure_ink(text_system, charge_font, px(DPI_VALUE_SIZE), "30000")
            .map_or(1.0, |ink| f32::from(ink.advance).max(1.0));
        let dpi_value_size = DPI_VALUE_SIZE * ((value_width - 0.5) / value_advance).min(1.0);
        let row_layout = ChargeRowLayout::new(WIDTH - 32.0, dpi_badge_width);
        let close = div()
            .id("close")
            .track_focus(&self.close_focus)
            .size(px(26.0))
            .rounded(px(7.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_color(muted)
            .border_1()
            .border_color(transparent_black())
            .hover(move |style| style.bg(badge).text_color(foreground))
            .focus(move |style| style.border_color(accent).bg(badge))
            .on_click(cx.listener(|card, _, _, _| card.dismiss(DismissReason::CloseButton)))
            .tooltip(move |_, cx| cx.new(|_| CardTooltip(close_label.into())).into())
            .child(
                svg()
                    .path("icons/close.svg")
                    .size(px(16.0))
                    .text_color(muted),
            );

        let main = if let Some(percent) = view.percent {
            div()
                .relative()
                .w_full()
                .h(px(CHARGE_ROW_HEIGHT))
                .flex_shrink_0()
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .h_full()
                        .flex()
                        .items_center()
                        .font_family(METRIC_FONT_FAMILY)
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .flex_shrink_0()
                                .gap(px(3.0))
                                .child(
                                    div()
                                        .text_size(px(CHARGE_FONT_SIZE))
                                        .line_height(px(CHARGE_LINE_HEIGHT))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(percent.to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(PERCENT_FONT_SIZE))
                                        .text_color(muted)
                                        .child("%"),
                                ),
                        ),
                )
                .when_some(view.dpi.clone(), |main, dpi| {
                    main.child(
                        div()
                            .absolute()
                            .left(px(row_layout.dpi_left))
                            .top_0()
                            .h_full()
                            .w(px(dpi_badge_width))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .id("active-dpi")
                                    .mt(px(dpi_layout.top))
                                    .h(px(dpi_layout.height))
                                    .w_full()
                                    .flex_shrink_0()
                                    .px(px(DPI_HORIZONTAL_PADDING))
                                    .rounded(px(4.0))
                                    .border_1()
                                    .border_color(color(palette.border))
                                    .font_family(METRIC_FONT_FAMILY)
                                    .flex()
                                    .flex_col()
                                    .justify_center()
                                    .tooltip(move |_, cx| {
                                        cx.new(|_| CardTooltip(dpi_hint.into())).into()
                                    })
                                    .child(
                                        div()
                                            .w(px(label_width))
                                            .text_size(px(DPI_LABEL_SIZE))
                                            .line_height(px(18.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(muted)
                                            .child(dpi.label),
                                    )
                                    .child(
                                        div()
                                            .ml(px(value_left))
                                            .w(px(value_width))
                                            .text_right()
                                            .text_size(px(dpi_value_size))
                                            .line_height(px(12.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(dpi.value),
                                    ),
                            ),
                    )
                })
                .child(
                    div()
                        .absolute()
                        .left(px(row_layout.power_left))
                        .top_0()
                        .h_full()
                        .w(px(row_layout.power_width))
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(6.0))
                        .child(
                            div()
                                .w(px(15.0))
                                .h(px(17.0))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(view.tone == StatusTone::Charging, |element| {
                                    element.child(
                                        svg()
                                            .path("icons/bolt.svg")
                                            .size(px(15.0))
                                            .text_color(accent),
                                    )
                                })
                                .when(view.tone != StatusTone::Charging, |element| {
                                    element.child(
                                        div()
                                            .flex_shrink_0()
                                            .size(px(5.0))
                                            .rounded_full()
                                            .bg(accent),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .text_ellipsis()
                                .text_size(px(12.0))
                                .line_height(px(17.0))
                                .text_color(muted)
                                .child(view.primary),
                        ),
                )
        } else {
            div()
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(3.0))
                .h(px(56.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .line_height(px(17.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(view.primary),
                )
                .when(!view.secondary.is_empty(), |element| {
                    element.child(
                        div()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(muted)
                            .child(view.secondary),
                    )
                })
        };
        let connection_icon = match self.model.status {
            BatteryStatus::Ready(b) if b.connection == ConnectionKind::Wired => "icons/usb.svg",
            _ => "icons/receiver.svg",
        };
        div()
            .id("battery-card")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .size_full()
            .overflow_hidden()
            .rounded(px(12.0))
            .border_1()
            .border_color(color(palette.border))
            .bg(color(palette.background))
            .text_color(foreground)
            .font_family(font_family)
            .font_weight(FontWeight::NORMAL)
            .p(px(15.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(24.0))
                    .flex_shrink_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                svg()
                                    .path("icons/mouse.svg")
                                    .size(px(20.0))
                                    .text_color(muted),
                            )
                            .child(
                                div()
                                    .font_family(METRIC_FONT_FAMILY)
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Ninjutso Ten"),
                            )
                            .when_some(view.polling_rate.clone(), |header, label| {
                                header.child(
                                    div()
                                        .absolute()
                                        .left(px(ChargeRowLayout::RATE_CENTER - 32.0))
                                        .top_0()
                                        .w(px(64.0))
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .id("polling-rate")
                                                .font_family(METRIC_FONT_FAMILY)
                                                .flex()
                                                .items_center()
                                                .flex_shrink_0()
                                                .h(px(18.0))
                                                .px(px(5.0))
                                                .rounded(px(4.0))
                                                .border_1()
                                                .border_color(color(palette.border))
                                                .text_size(px(10.0))
                                                .line_height(px(14.0))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(muted)
                                                .tooltip(move |_, cx| {
                                                    cx.new(|_| {
                                                        CardTooltip(polling_rate_hint.into())
                                                    })
                                                    .into()
                                                })
                                                .child(label),
                                        ),
                                )
                            }),
                    )
                    .child(close),
            )
            .child(main)
            .child(
                div()
                    .w_full()
                    .h(px(4.0))
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(color(palette.meter_track))
                    .overflow_hidden()
                    .when_some(view.percent, |track, percent| {
                        track.child(
                            div()
                                .h_full()
                                .w(relative(f32::from(percent) / 100.0))
                                .rounded_full()
                                .bg(accent),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(14.0))
                    .text_size(px(11.0))
                    .line_height(px(14.0))
                    .text_color(muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .when(!view.connection.is_empty(), |element| {
                                element.child(
                                    svg().path(connection_icon).size(px(13.0)).text_color(muted),
                                )
                            })
                            .child(view.connection),
                    )
                    .when(view.percent.is_some(), |element| {
                        element.child(view.secondary)
                    }),
            )
    }
}

struct CardTooltip(SharedString);

impl Render for CardTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgb(0x30343b))
            .text_color(rgb(0xffffff))
            .text_size(px(12.0))
            .max_w(px(268.0))
            .child(self.0.clone())
    }
}
