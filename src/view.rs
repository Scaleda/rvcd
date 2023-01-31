use crate::wave::{WaveDataItem, WaveInfo};
use egui::{vec2, Align2, ScrollArea, Sense, Ui};

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq)]
#[serde(default)]
pub struct SignalView {
    pub id: u64,
    pub height: f32,
}
pub const SIGNAL_HEIGHT_DEFAULT: f32 = 30.0;
impl SignalView {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            height: SIGNAL_HEIGHT_DEFAULT,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WaveView {
    pub signals: Vec<SignalView>,
    pub range: (u64, u64),
}

impl Default for WaveView {
    fn default() -> Self {
        Self {
            signals: vec![],
            range: (0, 0),
        }
    }
}

impl WaveView {
    pub fn view_panel(&mut self, ui: &mut Ui, info: &Option<WaveInfo>, wave_data: &[WaveDataItem]) {
        if let Some(info) = info {
            if self.range.0 == 0 && self.range.1 == 0 {
                self.range = info.range;
            }
        }
        ScrollArea::vertical().show(ui, |ui| {
            egui::SidePanel::left("signals")
                .resizable(true)
                .show_inside(ui, |ui| {
                    if let Some(info) = info {
                        for signal in self.signals.iter() {
                            if let Some(name) = info.code_names.get(&signal.id) {
                                ui.scope(|ui| {
                                    ui.set_height(signal.height);
                                    ui.centered_and_justified(|ui| {
                                        ui.add(egui::Label::new(name).wrap(false));
                                    });
                                });
                            }
                        }
                    }
                });
            egui::CentralPanel::default().show_inside(ui, |ui| {
                if let Some(info) = info {
                    for signal in self.signals.iter() {
                        ui.scope(|ui| {
                            ui.set_height(signal.height);
                            ui.centered_and_justified(|ui| {
                                let (mut _response, painter) = ui.allocate_painter(
                                    ui.available_size_before_wrap(),
                                    Sense::hover(),
                                );
                                let items = wave_data.iter().filter(|i| i.id == signal.id);
                                let color = ui.visuals().strong_text_color();
                                let rect = ui.max_rect();
                                let mut it = items;
                                let mut item_last: Option<&WaveDataItem> = None;
                                while let Some(item) = it.next() {
                                    if let Some(item_last) = item_last {
                                        let text = item_last.value.to_string();
                                        let width = rect.right() - rect.left();
                                        let percent = ((item_last.timestamp - info.range.0) as f32)
                                            / ((self.range.1 - self.range.0) as f32);
                                        let pos = rect.left_center() + vec2(width * percent, 0.0);
                                        painter.text(
                                            pos,
                                            Align2::CENTER_CENTER,
                                            text,
                                            Default::default(),
                                            color,
                                        );
                                    }
                                    item_last = Some(item);
                                }
                            });
                        });
                    }
                }
            });
        });
    }
}
