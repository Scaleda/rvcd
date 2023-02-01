pub mod cursor;
pub mod signal;

use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::view::cursor::WaveCursor;
use crate::view::signal::{SignalView, SignalViewAlign, SignalViewMode, SIGNAL_HEIGHT_DEFAULT};
use crate::wave::{WaveDataItem, WaveDataValue, WaveInfo, WaveTimescaleUnit, WireValue};
use eframe::emath::Align;
use egui::{
    pos2, vec2, Align2, Color32, Direction, DragValue, FontId, Layout, PointerButton, Pos2, Rect,
    Response, Sense, Ui, Widget,
};
use egui_extras::{Column, TableBuilder};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::ops::RangeInclusive;
use std::sync::mpsc;
use tracing::{debug, info, warn};

const LINE_WIDTH: f32 = 1.5;
const TEXT_ROUND_OFFSET: f32 = 4.0;
const MIN_SIGNAL_WIDTH: f32 = 2.0;
const BG_MULTIPLY: f32 = 0.05;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(default)]
pub struct WaveView {
    pub signals: Vec<SignalView>,
    pub range: (u64, u64),
    pub align: SignalViewAlign,
    pub background: bool,
    pub show_text: bool,
    pub default_radix: Radix,
    #[serde(skip)]
    pub tx: Option<mpsc::Sender<RvcdMsg>>,
    pub cursors: Vec<WaveCursor>,
    pub marker: WaveCursor,
    pub marker_temp: WaveCursor,
    pub wave_width: f32,
    pub signal_font_size: f32,
}

impl Default for WaveView {
    fn default() -> Self {
        Self {
            signals: vec![],
            range: (0, 0),
            align: Default::default(),
            background: true,
            show_text: true,
            default_radix: Radix::Hex,
            tx: None,
            cursors: vec![],
            marker: WaveCursor::from_string("Main Cursor"),
            marker_temp: WaveCursor::from_string(""),
            wave_width: 100.0,
            signal_font_size: 12.0,
        }
    }
}

impl WaveView {
    pub fn new(tx: mpsc::Sender<RvcdMsg>) -> Self {
        Self {
            tx: Some(tx),
            ..Default::default()
        }
    }
    pub fn set_tx(&mut self, tx: mpsc::Sender<RvcdMsg>) {
        self.tx = Some(tx);
    }
    pub fn signals_clean_unavailable(&mut self, info: &WaveInfo) {
        let signals: Vec<SignalView> = self
            .signals
            .clone()
            .into_iter()
            .filter(|signal| info.code_name_width.contains_key(&signal.s.id))
            .collect();
        debug!("signals: {} => {}", self.signals.len(), signals.len());
        self.signals = signals;
    }
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
            if ui.button("🔄 Refresh").clicked() {
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
    pub fn x_to_pos(&self, x: f32) -> u64 {
        (x * (self.range.1 - self.range.0) as f32 / self.wave_width) as u64 + self.range.0
        // x as u64
    }
    pub fn pos_to_x(&self, pos: u64) -> f32 {
        (pos - self.range.0) as f32 * self.wave_width / (self.range.1 - self.range.0) as f32
        // pos as f32
    }
    pub fn pos_to_time(&self, timescale: &(u64, WaveTimescaleUnit), pos: u64) -> String {
        format!("{}{}", pos * timescale.0, timescale.1)
    }
    pub fn time_bar(&mut self, ui: &mut Ui, info: &WaveInfo) {
        let rect = ui.max_rect();
        let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());
        // allocate size for text
        let text_rect = painter.text(
            Pos2::ZERO,
            Align2::RIGHT_BOTTOM,
            "0",
            FontId::monospace(self.signal_font_size),
            Color32::TRANSPARENT,
        );
        painter.hline(
            rect.x_range(),
            rect.min.y + text_rect.height(),
            (LINE_WIDTH, Color32::GREEN.linear_multiply(BG_MULTIPLY)),
        );
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
        let mut wave_left: f32 = 0.0;
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
                        self.time_bar(ui, info);
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
            self.paint_cursor(ui, wave_left, info, &self.marker_temp);
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
                format!("⬅{}➡", time),
                Default::default(),
                ui.visuals().strong_text_color(),
            );
        }
    }
    pub fn paint_cursor(&self, ui: &mut Ui, offset: f32, info: &WaveInfo, cursor: &WaveCursor) {
        let paint_rect = ui.max_rect();
        let painter = ui.painter();
        if cursor.valid {
            let x = self.pos_to_x(cursor.pos) + offset;
            painter.vline(x, paint_rect.y_range(), (LINE_WIDTH, Color32::YELLOW));
            let paint_text = |text: String, offset_y: f32| {
                painter.text(
                    pos2(x, paint_rect.top() + offset_y),
                    Align2::LEFT_TOP,
                    text,
                    Default::default(),
                    Color32::BLACK,
                )
            };
            let time = self.pos_to_time(&info.timescale, cursor.pos);
            let time_rect = paint_text(time.to_string(), 0.0);
            painter.rect_filled(time_rect, 0.0, Color32::YELLOW);
            paint_text(time, 0.0);
            if !cursor.name.is_empty() {
                let name_rect = paint_text(cursor.name.to_string(), time_rect.height());
                painter.rect_filled(name_rect, 0.0, Color32::YELLOW);
                paint_text(cursor.name.to_string(), time_rect.height());
            }
        }
    }
    pub fn reset(&mut self) {
        info!("reset");
        self.range = (0, 0);
        self.signals.clear();
    }
}
