use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::Result;
use cpal::traits::StreamTrait;
use eframe::egui;

use crate::audio::{AudioEngine, AudioParams};
use crate::device;

struct DeviceEntry {
    name: String,
    device: cpal::Device,
}

const ALL_BUFFER_SIZES: &[u32] = &[16, 32, 64, 128, 256, 512, 1024];
const ALL_SAMPLE_RATES: &[u32] = &[44100, 48000, 96000];

// Cyberpunk palette
const BG: egui::Color32 = egui::Color32::from_rgb(10, 10, 18);
const PANEL: egui::Color32 = egui::Color32::from_rgb(18, 18, 30);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(25, 25, 42);
const CYAN: egui::Color32 = egui::Color32::from_rgb(0, 255, 220);
const MAGENTA: egui::Color32 = egui::Color32::from_rgb(255, 0, 170);
const DIM: egui::Color32 = egui::Color32::from_rgb(70, 70, 100);
const TEXT: egui::Color32 = egui::Color32::from_rgb(190, 190, 210);
const TEXT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(230, 230, 245);

const LOGO: &str = "> vibetone_";

fn setup_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = PANEL;
    style.visuals.window_stroke = egui::Stroke::new(1.0, DIM);
    style.visuals.popup_shadow = egui::Shadow {
        offset: [0, 2],
        blur: 8,
        spread: 0,
        color: egui::Color32::from_black_alpha(120),
    };

    // Inactive widgets
    style.visuals.widgets.inactive.bg_fill = SURFACE;
    style.visuals.widgets.inactive.weak_bg_fill = SURFACE;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, DIM);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT);

    // Hovered
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(32, 32, 52);
    style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(32, 32, 52);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, CYAN);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, CYAN);

    // Active
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(40, 40, 60);
    style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(40, 40, 60);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, MAGENTA);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, MAGENTA);

    // Non-interactive
    style.visuals.widgets.noninteractive.bg_fill = PANEL;
    style.visuals.widgets.noninteractive.weak_bg_fill = PANEL;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, DIM);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;

    // Open (dropdowns)
    style.visuals.widgets.open.bg_fill = egui::Color32::from_rgb(28, 28, 48);
    style.visuals.widgets.open.weak_bg_fill = egui::Color32::from_rgb(28, 28, 48);
    style.visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, CYAN);
    style.visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, CYAN);

    // Selection highlight
    style.visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(0, 255, 220, 30);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, CYAN);

    // Separator
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, DIM);

    // Spacing
    style.spacing.item_spacing = egui::vec2(8.0, 5.0);
    style.spacing.button_padding = egui::vec2(20.0, 6.0);
    style.spacing.slider_width = 180.0;

    ctx.set_style(style);
}

struct VibetoneApp {
    inputs: Vec<DeviceEntry>,
    outputs: Vec<DeviceEntry>,
    selected_input: usize,
    selected_output: usize,
    buffer_size: u32,
    sample_rate: u32,
    volume: f32,
    noise_gate: bool,
    noise_gate_threshold: f32,
    available_buffer_sizes: Vec<u32>,
    available_sample_rates: Vec<u32>,
    voice_filter: bool,
    engine: Option<AudioEngine>,
    params_handle: Option<Arc<AudioParams>>,
    status: String,
    error: Option<String>,
    style_init: bool,
}

impl VibetoneApp {
    fn new() -> Self {
        let host = device::host();
        let inputs: Vec<DeviceEntry> = device::input_device_list(&host)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, name, device)| DeviceEntry { name, device })
            .collect();
        let outputs: Vec<DeviceEntry> = device::output_device_list(&host)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, name, device)| DeviceEntry { name, device })
            .collect();

        let (available_buffer_sizes, available_sample_rates) =
            if !inputs.is_empty() && !outputs.is_empty() {
                let inp = &inputs[0].device;
                let out = &outputs[0].device;
                (
                    device::supported_buffer_sizes(inp, out, ALL_BUFFER_SIZES),
                    device::supported_sample_rates(inp, out, ALL_SAMPLE_RATES),
                )
            } else {
                (ALL_BUFFER_SIZES.to_vec(), ALL_SAMPLE_RATES.to_vec())
            };

        let buffer_size = if available_buffer_sizes.contains(&64) {
            64
        } else {
            available_buffer_sizes.first().copied().unwrap_or(64)
        };

        let sample_rate = if available_sample_rates.contains(&48000) {
            48000
        } else {
            available_sample_rates.first().copied().unwrap_or(48000)
        };

        Self {
            inputs,
            outputs,
            selected_input: 0,
            selected_output: 0,
            buffer_size,
            sample_rate,
            volume: 1.0,
            noise_gate: false,
            noise_gate_threshold: -36.0,
            available_buffer_sizes,
            available_sample_rates,
            voice_filter: true,
            engine: None,
            params_handle: None,
            status: "OFFLINE".into(),
            error: None,
            style_init: false,
        }
    }

    fn is_running(&self) -> bool {
        self.engine.is_some()
    }

    fn refresh_device_capabilities(&mut self) {
        if self.inputs.is_empty() || self.outputs.is_empty() {
            return;
        }
        let inp = &self.inputs[self.selected_input].device;
        let out = &self.outputs[self.selected_output].device;

        self.available_buffer_sizes =
            device::supported_buffer_sizes(inp, out, ALL_BUFFER_SIZES);
        if !self.available_buffer_sizes.contains(&self.buffer_size) {
            self.buffer_size = self.available_buffer_sizes.first().copied().unwrap_or(64);
        }

        self.available_sample_rates =
            device::supported_sample_rates(inp, out, ALL_SAMPLE_RATES);
        if !self.available_sample_rates.contains(&self.sample_rate) {
            self.sample_rate = self.available_sample_rates.first().copied().unwrap_or(48000);
        }
    }

    fn start(&mut self) {
        self.error = None;
        if self.inputs.is_empty() || self.outputs.is_empty() {
            self.error = Some("No audio devices available".into());
            return;
        }

        let input = &self.inputs[self.selected_input].device;
        let output = &self.outputs[self.selected_output].device;

        let (in_ch, out_ch) = match device::negotiate_config(input, output) {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(format!("{e}"));
                return;
            }
        };

        let (engine, params) = match AudioEngine::build(
            input,
            output,
            self.sample_rate,
            self.buffer_size,
            in_ch,
            out_ch,
            self.volume,
        ) {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(format!("{e}"));
                return;
            }
        };

        if let Err(e) = engine.input_stream.play() {
            self.error = Some(format!("Input stream: {e}"));
            return;
        }
        if let Err(e) = engine.output_stream.play() {
            self.error = Some(format!("Output stream: {e}"));
            return;
        }

        self.params_handle = Some(params);
        self.engine = Some(engine);
        self.status = "LIVE".into();
    }

    fn stop(&mut self) {
        self.engine = None;
        self.params_handle = None;
        self.status = "OFFLINE".into();
    }

    fn sync_params(&self) {
        let Some(p) = &self.params_handle else {
            return;
        };
        p.volume.store(self.volume);
        p.noise_gate_enabled
            .store(self.noise_gate, Ordering::Relaxed);
        p.noise_gate_threshold.store(self.noise_gate_threshold);
        p.highpass_enabled
            .store(self.voice_filter, Ordering::Relaxed);
        p.lowpass_enabled
            .store(self.voice_filter, Ordering::Relaxed);
    }

    fn section_label(ui: &mut egui::Ui, text: &str) {
        ui.label(
            egui::RichText::new(text)
                .color(DIM)
                .size(10.0)
                .strong(),
        );
    }

    fn neon_separator(ui: &mut egui::Ui, color: egui::Color32) {
        let available = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(available, 1.0),
            egui::Sense::hover(),
        );
        ui.painter().line_segment(
            [rect.left_center(), rect.right_center()],
            egui::Stroke::new(0.5, color),
        );
    }
}

impl eframe::App for VibetoneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.style_init {
            setup_style(ctx);
            self.style_init = true;
        }

        let running = self.is_running();
        let accent = if running { CYAN } else { MAGENTA };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);

            // ── Logo ──
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(LOGO)
                        .monospace()
                        .color(accent)
                        .size(28.0)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("hear yourself vibe")
                        .color(DIM)
                        .size(11.0),
                );
            });

            ui.add_space(6.0);
            Self::neon_separator(ui, accent);
            ui.add_space(4.0);

            // ── Routing ──
            Self::section_label(ui, "ROUTING");
            ui.add_space(2.0);

            let prev_input = self.selected_input;
            let prev_output = self.selected_output;

            ui.add_enabled_ui(!running, |ui| {
                egui::Grid::new("routing")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("IN").color(CYAN).strong().size(11.0));
                        let in_name = if self.inputs.is_empty() {
                            "No devices".into()
                        } else {
                            self.inputs[self.selected_input].name.clone()
                        };
                        egui::ComboBox::from_id_salt("in")
                            .selected_text(egui::RichText::new(&in_name).color(TEXT_BRIGHT))
                            .width(310.0)
                            .show_ui(ui, |ui| {
                                for (i, e) in self.inputs.iter().enumerate() {
                                    ui.selectable_value(&mut self.selected_input, i, &e.name);
                                }
                            });
                        ui.end_row();

                        ui.label(egui::RichText::new("OUT").color(MAGENTA).strong().size(11.0));
                        let out_name = if self.outputs.is_empty() {
                            "No devices".into()
                        } else {
                            self.outputs[self.selected_output].name.clone()
                        };
                        egui::ComboBox::from_id_salt("out")
                            .selected_text(egui::RichText::new(&out_name).color(TEXT_BRIGHT))
                            .width(310.0)
                            .show_ui(ui, |ui| {
                                for (i, e) in self.outputs.iter().enumerate() {
                                    ui.selectable_value(&mut self.selected_output, i, &e.name);
                                }
                            });
                        ui.end_row();
                    });

                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new("BUF").color(DIM).size(10.0));
                    egui::ComboBox::from_id_salt("buf")
                        .selected_text(
                            egui::RichText::new(format!("{}", self.buffer_size)).color(TEXT_BRIGHT),
                        )
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for &s in &self.available_buffer_sizes {
                                ui.selectable_value(&mut self.buffer_size, s, format!("{s}"));
                            }
                        });

                    ui.label(egui::RichText::new("RATE").color(DIM).size(10.0));
                    egui::ComboBox::from_id_salt("rate")
                        .selected_text(
                            egui::RichText::new(format!("{} Hz", self.sample_rate))
                                .color(TEXT_BRIGHT),
                        )
                        .width(90.0)
                        .show_ui(ui, |ui| {
                            for &r in &self.available_sample_rates {
                                ui.selectable_value(&mut self.sample_rate, r, format!("{r} Hz"));
                            }
                        });

                    let ms = self.buffer_size as f64 / self.sample_rate as f64 * 1000.0;
                    ui.label(
                        egui::RichText::new(format!("{ms:.1}ms"))
                            .color(accent)
                            .size(10.0),
                    );
                });
            });

            if self.selected_input != prev_input || self.selected_output != prev_output {
                self.refresh_device_capabilities();
            }

            ui.add_space(4.0);
            Self::neon_separator(ui, DIM);
            ui.add_space(4.0);

            // ── Controls ──
            Self::section_label(ui, "CONTROLS");
            ui.add_space(2.0);

            // Volume
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("VOL")
                        .color(CYAN)
                        .strong()
                        .size(11.0),
                );
                ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).show_value(false));
                ui.label(
                    egui::RichText::new(format!("{}%", (self.volume * 100.0) as u32))
                        .color(TEXT_BRIGHT)
                        .monospace()
                        .size(11.0),
                );
            });

            ui.add_space(2.0);

            // Noise gate
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.noise_gate, "");
                ui.label(egui::RichText::new("GATE").strong().size(11.0));
                if self.noise_gate {
                    ui.add(
                        egui::Slider::new(&mut self.noise_gate_threshold, -60.0..=-10.0)
                            .show_value(false),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.0}dB", self.noise_gate_threshold))
                            .color(TEXT_BRIGHT)
                            .monospace()
                            .size(11.0),
                    );
                }
            });

            // Voice filter
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.voice_filter, "");
                ui.label(egui::RichText::new("FILTER").strong().size(11.0));
                ui.label(
                    egui::RichText::new("100Hz-8kHz")
                        .color(DIM)
                        .size(10.0),
                );
            });

            ui.add_space(4.0);
            Self::neon_separator(ui, accent);
            ui.add_space(6.0);

            // ── Action ──
            ui.vertical_centered(|ui| {
                let btn_text;
                let btn_fill;
                let btn_stroke;

                if running {
                    btn_text = egui::RichText::new("    STOP    ")
                        .strong()
                        .size(16.0)
                        .color(MAGENTA);
                    btn_fill = egui::Color32::from_rgb(50, 10, 25);
                    btn_stroke = egui::Stroke::new(1.5, MAGENTA);
                } else {
                    btn_text = egui::RichText::new("    START    ")
                        .strong()
                        .size(16.0)
                        .color(CYAN);
                    btn_fill = egui::Color32::from_rgb(8, 40, 35);
                    btn_stroke = egui::Stroke::new(1.5, CYAN);
                };

                let btn = egui::Button::new(btn_text)
                    .fill(btn_fill)
                    .stroke(btn_stroke);

                let can_start = !self.inputs.is_empty() && !self.outputs.is_empty();
                let enabled = if running { true } else { can_start };

                if ui.add_enabled(enabled, btn).clicked() {
                    if running {
                        self.stop();
                    } else {
                        self.start();
                    }
                }

                ui.add_space(4.0);

                let (dot, status_color) = if running {
                    (">>", CYAN)
                } else {
                    ("--", DIM)
                };
                ui.label(
                    egui::RichText::new(format!("{dot} {} {dot}", self.status))
                        .color(status_color)
                        .monospace()
                        .strong()
                        .size(12.0),
                );

                if let Some(err) = &self.error {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(err.as_str())
                            .color(egui::Color32::from_rgb(255, 80, 80))
                            .size(11.0),
                    );
                }
            });
        });

        self.sync_params();
    }
}

pub fn run() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 440.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Vibetone",
        options,
        Box::new(|_cc| Ok(Box::new(VibetoneApp::new()))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
