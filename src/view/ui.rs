use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::view::signal::{SIGNAL_HEIGHT_DEFAULT, SignalView, SignalViewAlign, SignalViewMode};
use crate::view::{WaveView, BG_MULTIPLY, LINE_WIDTH, MIN_SIGNAL_WIDTH, TEXT_ROUND_OFFSET, UI_WIDTH_OFFSET};
use crate::wave::{WaveDataItem, WaveDataValue, WaveInfo, WireValue};
use egui::*;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::ops::RangeInclusive;
use egui_extras::{Column, TableBuilder};
use tracing::*;

impl WaveView {
    pub fn menu(&mut self, ui: &mut Ui) {
        ui.menu_button("View", |ui| {
            ui.menu_button(format!("Default Radix: {:?}", self.default_radix), |ui| {
                use Radix::*;
                let data = [Hex, Oct, Dec, Bin];
                data.into_iter().for_each(|r| {
                    if ui.button(format!("{:?}", r)).clicked() {
                        self.default_radix = r;
                        ui.close_menu();
                    }
                });
            });
            ui.menu_button(format!("Align: {:?}", self.align), |ui| {
                use SignalViewAlign::*;
                let data = [Left, Center, Right];
                data.into_iter().for_each(|a| {
                    if ui.button(format!("{:?}", a)).clicked() {
                        self.align = a;
                        ui.close_menu();
                    }
                });
            });
            if ui.checkbox(&mut self.background, "Background").clicked() {
                ui.close_menu();
            }
            if ui.checkbox(&mut self.show_text, "Show Text").clicked() {
                ui.close_menu();
            }
            ui.horizontal(|ui| {
                ui.label("Value font size ");
                DragValue::new(&mut self.signal_font_size)
                    .clamp_range(10.0..=20.0)
                    .speed(0.05)
                    .suffix(" px")
                    .ui(ui);
            });
        });
    }
    pub fn toolbar(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            if ui.button("⛔ Clear").clicked() {
                self.signals.clear();
            }
            if ui.button("🔄 Reload").clicked() {
                if let Some(tx) = &self.tx {
                    debug!("reload msg sent");
                    tx.send(RvcdMsg::Reload).unwrap();
                } else {
                    warn!("no tx in view!");
                }
            }
        });
    }
    fn ui_signal_wave(
        &self,
        signal: &SignalView,
        wave_data: &[WaveDataItem],
        info: &WaveInfo,
        ui: &mut Ui,
    ) -> Response {
        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());
        let items = wave_data.iter().filter(|i| i.id == signal.s.id);
        let text_color = ui.visuals().strong_text_color();
        let signal_rect = response.rect;
        let mut it = items;
        let mut item_last: Option<&WaveDataItem> = None;
        let mut ignore_x_start = -1.0;
        let mut ignore_has_x = false;
        let mut paint_signal = |item_now: &WaveDataItem, item_next: &WaveDataItem| {
            let single: bool = match &item_now.value {
                WaveDataValue::Comp(_) => {
                    let d = ("".to_string(), 0);
                    let (_v, w) = info.code_name_width.get(&signal.s.id).unwrap_or(&d);
                    *w == 1
                }
                WaveDataValue::Raw(v) => v.len() == 1,
            };
            let width = signal_rect.width();
            let height = signal_rect.height();
            let percent_rect_left =
                (item_now.timestamp - info.range.0) as f32 / (self.range.1 - self.range.0) as f32;
            let percent_rect_right =
                (item_next.timestamp - info.range.0) as f32 / (self.range.1 - self.range.0) as f32;
            let percent_text = (((item_now.timestamp + item_next.timestamp) as f32 / 2.0)
                - info.range.0 as f32)
                / (self.range.1 - self.range.0) as f32;
            let rect = Rect::from_min_max(
                pos2(
                    signal_rect.left() + width * percent_rect_left,
                    signal_rect.top(),
                ),
                pos2(
                    signal_rect.left() + width * percent_rect_right,
                    signal_rect.top() + height,
                ),
            );
            let radix = match &signal.mode {
                SignalViewMode::Default => self.default_radix.clone(),
                SignalViewMode::Number(r) => r.clone(),
                SignalViewMode::Analog => Radix::Hex,
            };
            let text = item_now.value.as_radix(radix);
            if rect.width() > MIN_SIGNAL_WIDTH {
                if ignore_x_start >= 0.0 {
                    // paint a rect as ignored data
                    painter.rect_filled(
                        Rect::from_x_y_ranges(
                            RangeInclusive::new(ignore_x_start, rect.left()),
                            rect.y_range(),
                        ),
                        0.0,
                        if ignore_has_x {
                            Color32::DARK_RED
                        } else {
                            Color32::GREEN
                        },
                    );
                    ignore_x_start = -1.0;
                    ignore_has_x = false;
                }
                let paint_x = || {
                    painter.rect(
                        rect,
                        0.0,
                        if self.background {
                            Color32::DARK_RED.linear_multiply(BG_MULTIPLY)
                        } else {
                            Color32::TRANSPARENT
                        },
                        (LINE_WIDTH, Color32::RED),
                    )
                };
                let paint_z = || painter.rect_stroke(rect, 0.0, (LINE_WIDTH, Color32::DARK_RED));
                if single {
                    let value = match &item_now.value {
                        WaveDataValue::Comp(v) => match BigUint::from_bytes_le(v).is_one() {
                            true => WireValue::V1,
                            false => WireValue::V0,
                        },
                        WaveDataValue::Raw(v) => v[0],
                    };
                    match value {
                        WireValue::V0 => {
                            painter.hline(
                                rect.x_range(),
                                rect.bottom(),
                                (LINE_WIDTH, Color32::GREEN),
                            );
                            painter.vline(
                                rect.left(),
                                rect.y_range(),
                                (LINE_WIDTH, Color32::GREEN),
                            );
                        }
                        WireValue::V1 => {
                            painter.hline(rect.x_range(), rect.top(), (LINE_WIDTH, Color32::GREEN));
                            painter.vline(
                                rect.left(),
                                rect.y_range(),
                                (LINE_WIDTH, Color32::GREEN),
                            );
                        }
                        WireValue::X => paint_x(),
                        WireValue::Z => paint_z(),
                    };
                } else {
                    let number: Option<BigUint> = (&item_now.value).into();
                    if text.contains('x') {
                        paint_x();
                    } else {
                        if text.contains('z') {
                            paint_z();
                        } else {
                            match number {
                                Some(n) if n.is_zero() => {
                                    painter.hline(
                                        rect.x_range(),
                                        rect.bottom(),
                                        (LINE_WIDTH, Color32::GREEN),
                                    );
                                }
                                _ => {
                                    painter.rect(
                                        rect,
                                        0.0,
                                        if self.background {
                                            Color32::GREEN.linear_multiply(BG_MULTIPLY)
                                        } else {
                                            Color32::TRANSPARENT
                                        },
                                        (LINE_WIDTH, Color32::GREEN),
                                    );
                                }
                            }
                        }
                    }
                    if self.show_text {
                        let text_min_rect = painter.text(
                            Pos2::ZERO,
                            Align2::RIGHT_BOTTOM,
                            "+",
                            FontId::monospace(self.signal_font_size),
                            Color32::TRANSPARENT,
                        );
                        if rect.width() >= text_min_rect.width() + TEXT_ROUND_OFFSET {
                            let pos = match self.align {
                                SignalViewAlign::Left => {
                                    rect.left_center() + vec2(TEXT_ROUND_OFFSET, 0.0)
                                }
                                SignalViewAlign::Center => {
                                    rect.left_center() + vec2(width * percent_text, 0.0)
                                }
                                SignalViewAlign::Right => {
                                    rect.right_center() - vec2(TEXT_ROUND_OFFSET, 0.0)
                                }
                            };
                            // pre-paint to calculate size
                            let text_rect = painter.text(
                                pos,
                                match self.align {
                                    SignalViewAlign::Left => Align2::LEFT_CENTER,
                                    SignalViewAlign::Center => Align2::CENTER_CENTER,
                                    SignalViewAlign::Right => Align2::RIGHT_CENTER,
                                },
                                text.as_str(),
                                FontId::monospace(self.signal_font_size),
                                Color32::TRANSPARENT,
                            );
                            let paint_text =
                                if rect.width() >= text_rect.width() + TEXT_ROUND_OFFSET {
                                    text
                                } else {
                                    let text_mono_width = text_rect.width() / text.len() as f32;
                                    let text_len = text.len();
                                    let remains = &text[0..(text_len
                                        - ((text_rect.width() + TEXT_ROUND_OFFSET - rect.width())
                                            / text_mono_width)
                                            as usize)];
                                    if remains.len() <= 1 {
                                        "+".to_string()
                                    } else {
                                        let len = remains.len();
                                        format!("{}+", &remains[0..(len - 2)])
                                    }
                                };
                            painter.text(
                                pos,
                                match self.align {
                                    SignalViewAlign::Left => Align2::LEFT_CENTER,
                                    SignalViewAlign::Center => Align2::CENTER_CENTER,
                                    SignalViewAlign::Right => Align2::RIGHT_CENTER,
                                },
                                paint_text,
                                // Default::default(),
                                FontId::monospace(self.signal_font_size),
                                text_color,
                            );
                        }
                    }
                }
            } else {
                // ignore this paint, record start pos
                if ignore_x_start < 0.0 {
                    ignore_x_start = rect.left();
                }
                if text.contains('x') || text.contains('z') {
                    ignore_has_x = true;
                }
            }
        };
        while let Some(item) = it.next() {
            if let Some(item_last) = item_last {
                paint_signal(item_last, item);
            }
            item_last = Some(item);
        }
        if let Some(item_last) = item_last {
            paint_signal(
                item_last,
                &WaveDataItem {
                    timestamp: info.range.1,
                    ..WaveDataItem::default()
                },
            );
        }
        // draw last
        if ignore_x_start >= 0.0 {
            painter.rect_filled(
                Rect::from_x_y_ranges(
                    RangeInclusive::new(ignore_x_start, signal_rect.right()),
                    signal_rect.y_range(),
                ),
                0.0,
                if ignore_has_x {
                    Color32::DARK_RED
                } else {
                    Color32::GREEN
                },
            )
        }
        response
    }
    fn ui_signal_label(&self, signal: &SignalView, ui: &mut Ui) {
        let text = signal.s.to_string();
        ui.scope(|ui| {
            ui.set_height(signal.height);
            ui.centered_and_justified(|ui| {
                ui.add(egui::Label::new(text).wrap(false));
            });
        });
    }
    pub fn panel(&mut self, ui: &mut Ui, info: &Option<WaveInfo>, wave_data: &[WaveDataItem]) {
        if let Some(info) = info {
            if self.range.0 == 0 && self.range.1 == 0 {
                self.range = info.range;
            }
        }
        egui::TopBottomPanel::top("wave_top")
            .resizable(false)
            .show_inside(ui, |ui| {
                self.toolbar(ui);
            });
        // bugs by: https://github.com/emilk/egui/issues/2430
        let use_rect = ui.max_rect();
        const DEFAULT_MIN_SIGNAL_WIDTH: f32 = 150.0;
        let fix_width = f32::max(
            self.signals
                .iter()
                .map(|x| x.s.name.len())
                .max()
                .unwrap_or(0) as f32
                * 8.0,
            DEFAULT_MIN_SIGNAL_WIDTH,
        );
        self.wave_width = use_rect.width() - fix_width;
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            // .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .cell_layout(egui::Layout::centered_and_justified(Direction::TopDown))
            .column(Column::exact(fix_width).resizable(false))
            .column(Column::exact(self.wave_width).resizable(false));
        // .column(Column::auto())
        // .column(Column::remainder());
        let mut wave_left: f32 = fix_width + use_rect.left() + UI_WIDTH_OFFSET;
        let mut pos = None;
        let mut drag_started = false;
        let mut drag_release = false;
        let mut drag_by_primary = false;
        let mut drag_by_secondary = false;
        let mut drag_by_middle = false;
        table
            .header(SIGNAL_HEIGHT_DEFAULT, |mut header| {
                header.col(|ui| {
                    if let Some(info) = info {
                        ui.strong(format!(
                            "Time #{}~#{} {}{}",
                            info.range.0, info.range.1, info.timescale.0, info.timescale.1
                        ));
                    }
                });
                header.col(|ui| {
                    if let Some(info) = info {
                        self.time_bar(ui, info, wave_left);
                    }
                });
            })
            .body(|body| {
                body.heterogeneous_rows(
                    self.signals.iter().map(|x| x.height),
                    |row_index, mut row| {
                        let signal = self.signals.get(row_index);
                        if let Some(signal) = signal {
                            row.col(|ui| self.ui_signal_label(signal, ui));
                            row.col(|ui| {
                                if let Some(info) = info {
                                    let response = self.ui_signal_wave(signal, wave_data, info, ui);
                                    wave_left = ui.available_rect_before_wrap().left();
                                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                                        pos = Some(pos2(pointer_pos.x - wave_left, pointer_pos.y));
                                        drag_started = response.drag_started();
                                        drag_release = response.drag_released();
                                        if response.dragged_by(PointerButton::Primary) {
                                            drag_by_primary = true;
                                        }
                                        if response.dragged_by(PointerButton::Secondary) {
                                            drag_by_secondary = true;
                                        }
                                        if response.dragged_by(PointerButton::Middle) {
                                            drag_by_middle = true;
                                        }
                                    }
                                }
                            });
                        }
                    },
                );
            });
        // info!("fix_width = {}, ui left = {}, wave_left = {}", fix_width, ui.max_rect().left(), wave_left);
        // info!("(fix_width + ui left) - wave_left = {}", fix_width + ui.max_rect().left() - wave_left);
        if let Some(pos) = pos {
            let painter = ui.painter();
            painter.text(
                pos + vec2(wave_left, 0.0),
                Align2::RIGHT_BOTTOM,
                format!("{:?}", pos),
                Default::default(),
                Color32::YELLOW,
            );
            if drag_by_primary {
                self.marker_temp
                    .set_pos_valid(self.x_to_pos(pos.x).clamp(self.range.0, self.range.1));
            }
            if drag_release && self.marker_temp.valid {
                self.marker
                    .set_pos_valid(self.marker_temp.pos.clamp(self.range.0, self.range.1));
            }
            if !drag_by_primary {
                self.marker_temp.valid = false;
            }
        }
        if let Some(info) = info {
            self.paint_span(ui, wave_left, info, pos);
            self.paint_cursor(ui, wave_left, info, &self.marker);
            if self.marker_temp.valid {
                self.paint_cursor(ui, wave_left, info, &self.marker_temp);
            }
            for cursor in &self.cursors {
                self.paint_cursor(ui, wave_left, info, cursor);
            }
        }
    }
    pub fn paint_span(&self, ui: &mut Ui, offset: f32, info: &WaveInfo, pos: Option<Pos2>) {
        let paint_rect = ui.max_rect();
        let painter = ui.painter();
        if self.marker.valid && self.marker_temp.valid {
            let (a, b) = if self.marker.pos < self.marker_temp.pos {
                (&self.marker, &self.marker_temp)
            } else {
                (&self.marker_temp, &self.marker)
            };
            let (x_a, x_b) = (self.pos_to_x(a.pos) + offset, self.pos_to_x(b.pos) + offset);
            let rect = Rect::from_min_max(pos2(x_a, paint_rect.min.y), pos2(x_b, paint_rect.max.y));
            painter.rect(
                rect,
                0.0,
                Color32::BLUE.linear_multiply(BG_MULTIPLY),
                (LINE_WIDTH, Color32::BLUE),
            );
            let y = match pos {
                None => paint_rect.top(),
                Some(pos) => pos.y,
            };
            painter.hline(
                RangeInclusive::new(x_a, x_b),
                y,
                (LINE_WIDTH, Color32::BLUE),
            );
            let time = self.pos_to_time(&info.timescale, b.pos - a.pos);
            painter.text(
                pos2((x_a + x_b) / 2.0, y),
                Align2::CENTER_BOTTOM,
                if self.marker.pos <= self.marker_temp.pos {
                    format!("+{}", time)
                } else {
                    format!("-{}", time)
                },
                Default::default(),
                ui.visuals().strong_text_color(),
            );
        }
    }
}
