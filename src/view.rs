use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::wave::{WaveDataItem, WaveDataValue, WaveInfo, WaveSignalInfo, WireValue};
use eframe::emath::Align;
use egui::{pos2, vec2, Align2, Color32, Direction, Layout, Rect, Sense, Ui};
use egui_extras::{Column, TableBuilder};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::ops::RangeInclusive;
use std::sync::mpsc;
use tracing::{debug, info, warn};

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default)]
pub enum SignalViewMode {
    #[default]
    Default,
    Number(Radix),
    Analog,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Default, Debug)]
pub enum SignalViewAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Debug, Clone)]
pub struct SignalView {
    pub s: WaveSignalInfo,
    pub height: f32,
    pub mode: SignalViewMode,
}
pub const SIGNAL_HEIGHT_DEFAULT: f32 = 30.0;
impl SignalView {
    pub fn from_id(id: u64, info: &WaveInfo) -> Self {
        let d = ("unknown".to_string(), 0);
        let name_width = info.code_name_width.get(&id).unwrap_or(&d).clone();
        Self {
            s: WaveSignalInfo {
                id,
                name: name_width.0,
                width: name_width.1,
            },
            height: SIGNAL_HEIGHT_DEFAULT,
            mode: Default::default(),
        }
    }
}

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
    ) {
        const LINE_WIDTH: f32 = 1.5;
        const MIN_TEXT_WIDTH: f32 = 6.0;
        const MIN_SIGNAL_WIDTH: f32 = 2.0;
        const BG_MULTIPLY: f32 = 0.05;
        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::hover());
        let items = wave_data.iter().filter(|i| i.id == signal.s.id);
        let color = ui.visuals().strong_text_color();
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
                    let text = item_now.value.to_string();
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
                    if rect.width() > MIN_TEXT_WIDTH && rect.width() > (text.len() * 8) as f32 {
                        let pos = match self.align {
                            SignalViewAlign::Left => rect.left_center() + vec2(4.0, 0.0),
                            SignalViewAlign::Center => {
                                rect.left_center() + vec2(width * percent_text, 0.0)
                            }
                            SignalViewAlign::Right => rect.right_center(),
                        };
                        if self.show_text {
                            painter.text(
                                pos,
                                match self.align {
                                    SignalViewAlign::Left => Align2::LEFT_CENTER,
                                    SignalViewAlign::Center => Align2::CENTER_CENTER,
                                    SignalViewAlign::Right => Align2::RIGHT_CENTER,
                                },
                                text,
                                Default::default(),
                                color,
                            );
                        }
                    }
                }
            } else {
                // ignore this paint, record start pos
                if ignore_x_start < 0.0 {
                    ignore_x_start = rect.left();
                }
                let text = item_now.value.to_string();
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
        let rect = ui.max_rect();
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
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            // .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .cell_layout(egui::Layout::centered_and_justified(Direction::TopDown))
            .column(Column::exact(fix_width).resizable(true))
            .column(Column::exact(rect.width() - fix_width).resizable(false));
        // .column(Column::auto())
        // .column(Column::remainder());
        table
            .header(SIGNAL_HEIGHT_DEFAULT, |mut header| {
                let mut width = 0.0;
                header.col(|ui| {
                    width = ui.available_width();
                    if let Some(info) = info {
                        ui.strong(format!(
                            "Time #{}~#{} {}{}",
                            info.range.0, info.range.1, info.timescale.0, info.timescale.1
                        ));
                    }
                });
                header.col(|ui| {
                    ui.set_width(width);
                    ui.strong("Wave");
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
                                    self.ui_signal_wave(signal, wave_data, info, ui);
                                }
                            });
                        }
                    },
                );
            });
    }
    pub fn reset(&mut self) {
        info!("reset");
        self.range = (0, 0);
        self.signals.clear();
    }
}
