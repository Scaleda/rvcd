#![cfg(not(target_arch = "wasm32"))]

use egui::text::LayoutJob;
use egui::text_edit::TextEditOutput;

/// View some code with syntax highlighting and selection.
pub fn code_view_ui(ui: &mut egui::Ui, code: &mut String, offset: Option<usize>) -> TextEditOutput {
    let language = "rs";
    // let theme = CodeTheme::from_memory(ui.ctx());
    let theme = CodeTheme::default();

    let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
        let layout_job = highlight(ui.ctx(), &theme, string, language);
        // layout_job.wrap.max_width = wrap_width; // no wrapping
        ui.fonts(|f| f.layout_job(layout_job))
    };

    let output = egui::TextEdit::multiline(code)
        .font(egui::TextStyle::Monospace) // for cursor height
        .code_editor()
        .desired_rows(1)
        .lock_focus(true)
        .desired_width(f32::INFINITY)
        .layouter(&mut layouter)
        .show(ui);

    let text_edit_id = output.response.id;
    if let Some(offset) = offset {
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id) {
            let ccursor = egui::text::CCursor::new(offset);
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
            state.store(ui.ctx(), text_edit_id);
            ui.ctx().memory_mut(|mem| mem.request_focus(text_edit_id)); // give focus back to the [`TextEdit`].
        }
    }
    output
}

/// Memoized Code highlighting
pub fn highlight(ctx: &egui::Context, theme: &CodeTheme, code: &str, language: &str) -> LayoutJob {
    impl egui::util::cache::ComputerMut<(&CodeTheme, &str, &str), LayoutJob> for Highlighter {
        fn compute(&mut self, (theme, code, lang): (&CodeTheme, &str, &str)) -> LayoutJob {
            self.highlight(theme, code, lang)
        }
    }

    type HighlightCache = egui::util::cache::FrameCache<LayoutJob, Highlighter>;

    ctx.memory_mut(|mem| {
        mem.caches
            .cache::<HighlightCache>()
            .get((theme, code, language))
    })
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize, enum_map::Enum)]
enum TokenType {
    Comment,
    Keyword,
    Literal,
    StringLiteral,
    Punctuation,
    Whitespace,
}

#[derive(Clone, Hash, PartialEq)]
// #[derive(serde::Deserialize, serde::Serialize)]
// #[serde(default)]
pub struct CodeTheme {
    dark_mode: bool,
    formats: enum_map::EnumMap<TokenType, egui::TextFormat>,
}

impl Default for CodeTheme {
    fn default() -> Self {
        Self::dark()
    }
}

// impl CodeTheme {
//     pub fn from_style(style: &egui::Style) -> Self {
//         if style.visuals.dark_mode {
//             Self::dark()
//         } else {
//             Self::light()
//         }
//     }
//
//     pub fn from_memory(ctx: &egui::Context) -> Self {
//         if ctx.style().visuals.dark_mode {
//             ctx.data_mut(|d| {
//                 d.get_persisted(egui::Id::new("dark"))
//                     .unwrap_or_else(CodeTheme::dark)
//             })
//         } else {
//             ctx.data_mut(|d| {
//                 d.get_persisted(egui::Id::new("light"))
//                     .unwrap_or_else(CodeTheme::light)
//             })
//         }
//     }
//
//     pub fn store_in_memory(self, ctx: &egui::Context) {
//         if self.dark_mode {
//             ctx.data_mut(|d| d.insert_persisted(egui::Id::new("dark"), self));
//         } else {
//             ctx.data_mut(|d| d.insert_persisted(egui::Id::new("light"), self));
//         }
//     }
// }

impl CodeTheme {
    pub fn dark() -> Self {
        let font_id = egui::FontId::monospace(10.0);
        use egui::{Color32, TextFormat};
        Self {
            dark_mode: true,
            formats: enum_map::enum_map![
                TokenType::Comment => TextFormat::simple(font_id.clone(), Color32::from_gray(120)),
                TokenType::Keyword => TextFormat::simple(font_id.clone(), Color32::from_rgb(255, 100, 100)),
                TokenType::Literal => TextFormat::simple(font_id.clone(), Color32::from_rgb(87, 165, 171)),
                TokenType::StringLiteral => TextFormat::simple(font_id.clone(), Color32::from_rgb(109, 147, 226)),
                TokenType::Punctuation => TextFormat::simple(font_id.clone(), Color32::LIGHT_GRAY),
                TokenType::Whitespace => TextFormat::simple(font_id.clone(), Color32::TRANSPARENT),
            ],
        }
    }

    pub fn light() -> Self {
        let font_id = egui::FontId::monospace(10.0);
        use egui::{Color32, TextFormat};
        Self {
            dark_mode: false,
            formats: enum_map::enum_map![
                TokenType::Comment => TextFormat::simple(font_id.clone(), Color32::GRAY),
                TokenType::Keyword => TextFormat::simple(font_id.clone(), Color32::from_rgb(235, 0, 0)),
                TokenType::Literal => TextFormat::simple(font_id.clone(), Color32::from_rgb(153, 134, 255)),
                TokenType::StringLiteral => TextFormat::simple(font_id.clone(), Color32::from_rgb(37, 203, 105)),
                TokenType::Punctuation => TextFormat::simple(font_id.clone(), Color32::DARK_GRAY),
                TokenType::Whitespace => TextFormat::simple(font_id.clone(), Color32::TRANSPARENT),
            ],
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_top(|ui| {
            let selected_id = egui::Id::NULL;
            let mut selected_tt: TokenType =
                ui.data_mut(|d| *d.get_persisted_mut_or(selected_id, TokenType::Comment));

            ui.vertical(|ui| {
                ui.set_width(150.0);
                egui::widgets::global_dark_light_mode_buttons(ui);

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.scope(|ui| {
                    for (tt, tt_name) in [
                        (TokenType::Comment, "// comment"),
                        (TokenType::Keyword, "keyword"),
                        (TokenType::Literal, "literal"),
                        (TokenType::StringLiteral, "\"string literal\""),
                        (TokenType::Punctuation, "punctuation ;"),
                        // (TokenType::Whitespace, "whitespace"),
                    ] {
                        let format = &mut self.formats[tt];
                        ui.style_mut().override_font_id = Some(format.font_id.clone());
                        ui.visuals_mut().override_text_color = Some(format.color);
                        ui.radio_value(&mut selected_tt, tt, tt_name);
                    }
                });

                let reset_value = if self.dark_mode {
                    CodeTheme::dark()
                } else {
                    CodeTheme::light()
                };

                if ui
                    .add_enabled(*self != reset_value, egui::Button::new("Reset theme"))
                    .clicked()
                {
                    *self = reset_value;
                }
            });

            ui.add_space(16.0);

            ui.data_mut(|d| d.insert_persisted(selected_id, selected_tt));

            egui::Frame::group(ui.style())
                .inner_margin(egui::Vec2::splat(2.0))
                .show(ui, |ui| {
                    // ui.group(|ui| {
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Small);
                    ui.spacing_mut().slider_width = 128.0; // Controls color picker size
                    egui::widgets::color_picker::color_picker_color32(
                        ui,
                        &mut self.formats[selected_tt].color,
                        egui::color_picker::Alpha::Opaque,
                    );
                });
        });
    }
}
#[derive(Default)]
struct Highlighter {}

impl Highlighter {
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn highlight(&self, theme: &CodeTheme, mut text: &str, _language: &str) -> LayoutJob {
        let mut job = LayoutJob::default();

        while !text.is_empty() {
            if text.starts_with("//") {
                let end = text.find('\n').unwrap_or(text.len());
                job.append(&text[..end], 0.0, theme.formats[TokenType::Comment].clone());
                text = &text[end..];
            } else if text.starts_with('"') {
                let end = text[1..]
                    .find('"')
                    .map(|i| i + 2)
                    .or_else(|| text.find('\n'))
                    .unwrap_or(text.len());
                job.append(
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::StringLiteral].clone(),
                );
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_alphanumeric()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_alphanumeric())
                    .map_or_else(|| text.len(), |i| i + 1);
                let word = &text[..end];
                let tt = if is_keyword(word) {
                    TokenType::Keyword
                } else {
                    TokenType::Literal
                };
                job.append(word, 0.0, theme.formats[tt].clone());
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_whitespace()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_whitespace())
                    .map_or_else(|| text.len(), |i| i + 1);
                job.append(
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::Whitespace].clone(),
                );
                text = &text[end..];
            } else {
                let mut it = text.char_indices();
                it.next();
                let end = it.next().map_or(text.len(), |(idx, _chr)| idx);
                job.append(
                    &text[..end],
                    0.0,
                    theme.formats[TokenType::Punctuation].clone(),
                );
                text = &text[end..];
            }
        }

        job
    }
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "always"
            | "and"
            | "assign"
            | "automatic"
            | "begin"
            | "buf"
            | "bufif0"
            | "bufif1"
            | "case"
            | "casex"
            | "casez"
            | "cmos"
            | "deassign"
            | "default"
            | "defparam"
            | "disable"
            | "edge"
            | "else"
            | "end"
            | "endcase"
            | "endfunction"
            | "endgenerate"
            | "endmodule"
            | "endprimitive"
            | "endspecify"
            | "endtable"
            | "endtask"
            | "event"
            | "for"
            | "force"
            | "forever"
            | "fork"
            | "function"
            | "generate"
            | "genvar"
            | "highz0"
            | "highz1"
            | "if"
            | "initial"
            | "inout"
            | "input"
            | "integer"
            | "join"
            | "large"
            | "localparam"
            | "macromodule"
            | "medium"
            | "module"
            | "nand"
            | "negedge"
            | "nmos"
            | "nor"
            | "not"
            | "notif0"
            | "notif1"
            | "or"
            | "output"
            | "parameter"
            | "pmos"
            | "posedge"
            | "primitive"
            | "pull0"
            | "pull1"
            | "pulldown"
            | "pullup"
            | "rcmos"
            | "real"
            | "realtime"
            | "reg"
            | "release"
            | "repeat"
            | "rnmos"
            | "rpmos"
            | "rtran"
            | "rtranif0"
            | "rtranif1"
            | "scalared"
            | "small"
            | "specify"
            | "specparam"
            | "strong0"
            | "strong1"
            | "supply0"
            | "supply1"
            | "table"
            | "task"
            | "time"
            | "tran"
            | "tranif0"
            | "tranif1"
            | "tri"
            | "tri0"
            | "tri1"
            | "triand"
            | "trior"
            | "trireg"
            | "unsigned"
            | "vectored"
            | "wait"
            | "wand"
            | "weak0"
            | "weak1"
            | "while"
            | "wire"
            | "wor"
            | "xnor"
            | "xor"
        // VERILOG_DIRECTIVES
            | "define"
            | "include"
            | "ifdef"
            | "endif"
            | "timescale"
    )
}
