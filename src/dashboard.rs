#![cfg(feature = "dashboard")]

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, FontId, Layout, RichText, Rounding, Sense, Stroke, Vec2};
use tokio::runtime::Runtime;
use tokio::sync::broadcast;

use crate::five_kernel_matrix::{FiveKernelMatrix, FiveKernelMatrixConfig, KernelIngress};
use crate::ikc::{IkcEvent, IkcMessage, KernelCommand, KernelId, MediaType};

pub const DASHBOARD_MEMORY_BUDGET_BYTES: usize = 50 * 1024 * 1024;
const MAX_CHAT_ENTRIES: usize = 128;
const MAX_EVENT_ENTRIES: usize = 256;
const MAX_MEDIA_FRAMES: usize = 96;
const MAX_PREVIEW_BYTES: usize = 512 * 1024;
const MAX_INPUT_BYTES: usize = 16 * 1024;

pub fn run_vortex_dashboard() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Vortex Dashboard",
        native_options,
        Box::new(|_cc| Ok(Box::new(VortexDashboardApp::new()))),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DashboardMode {
    IntelligentChatScenario,
    LivePreview,
    MediaProcessing,
}

impl DashboardMode {
    fn title(self) -> &'static str {
        match self {
            Self::IntelligentChatScenario => "Intelligent Chat & Scenario Writing",
            Self::LivePreview => "Live Preview Panel",
            Self::MediaProcessing => "Media Processing Windows",
        }
    }

    fn short(self) -> &'static str {
        match self {
            Self::IntelligentChatScenario => "Chat",
            Self::LivePreview => "Preview",
            Self::MediaProcessing => "Media",
        }
    }

    fn accent(self) -> Color32 {
        match self {
            Self::IntelligentChatScenario => Color32::from_rgb(114, 92, 255),
            Self::LivePreview => Color32::from_rgb(0, 180, 150),
            Self::MediaProcessing => Color32::from_rgb(242, 121, 53),
        }
    }

    fn soft(self) -> Color32 {
        match self {
            Self::IntelligentChatScenario => Color32::from_rgb(30, 26, 58),
            Self::LivePreview => Color32::from_rgb(16, 48, 43),
            Self::MediaProcessing => Color32::from_rgb(58, 33, 22),
        }
    }
}

#[derive(Clone, Debug)]
struct ChatEntry {
    role: &'static str,
    text: String,
}

#[derive(Clone, Debug)]
struct MediaFrameRecord {
    request_id: String,
    media_type: MediaType,
    frame_index: usize,
    total_frames: usize,
}

pub struct VortexDashboardApp {
    runtime: Runtime,
    matrix: Option<FiveKernelMatrix>,
    ingress: KernelIngress,
    events: broadcast::Receiver<IkcEvent>,
    mode: DashboardMode,
    session_id: String,
    chat_input: String,
    preview_prompt: String,
    media_prompt: String,
    chat_log: VecDeque<ChatEntry>,
    event_log: VecDeque<String>,
    media_frames: VecDeque<MediaFrameRecord>,
    preview_source: String,
    preview_status: String,
    last_repaint: Instant,
}

impl VortexDashboardApp {
    fn new() -> Self {
        let runtime = match Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[Dashboard] Failed to create Tokio runtime: {e}");
                std::process::exit(1);
            }
        };
        let matrix = runtime.block_on(async {
            FiveKernelMatrix::spawn(FiveKernelMatrixConfig {
                memory_budget_bytes: DASHBOARD_MEMORY_BUDGET_BYTES,
                ..FiveKernelMatrixConfig::default()
            })
        });
        let ingress = matrix.ingress();
        let events = matrix.subscribe_events();

        let mut chat_log = VecDeque::new();
        chat_log.push_back(ChatEntry {
            role: "system",
            text: "Vortex Dashboard connected to Kernel_01. Select a mode and submit work directly into the 5-Kernel Matrix.".to_string(),
        });

        Self {
            runtime,
            matrix: Some(matrix),
            ingress,
            events,
            mode: DashboardMode::IntelligentChatScenario,
            session_id: format!("dashboard_{}", std::process::id()),
            chat_input: String::new(),
            preview_prompt: "Create a responsive landing page for Vortex Atoms AI.".to_string(),
            media_prompt: "Generate a cinematic 12-frame abstract vortex animation.".to_string(),
            chat_log,
            event_log: VecDeque::new(),
            media_frames: VecDeque::new(),
            preview_source: default_preview_source(),
            preview_status: "Preview canvas ready; submit a website, app, or game request."
                .to_string(),
            last_repaint: Instant::now(),
        }
    }

    fn send_to_kernel_01(&mut self, text: impl Into<String>) {
        let mut text = text.into();
        clamp_string(&mut text, MAX_INPUT_BYTES);

        let session_id = self.session_id.clone();
        let tx = self.ingress.kernel_01.clone();
        let message = IkcMessage::new(
            KernelId::Kernel01UiInteraction,
            KernelId::Kernel01UiInteraction,
            KernelCommand::UiInput {
                session_id: session_id.clone(),
                text: text.clone(),
            },
        );

        self.chat_log.push_back(ChatEntry { role: "user", text });
        self.runtime.spawn(async move {
            let _ = tx.send(message).await;
        });
        self.trim_memory();
    }

    fn poll_kernel_events(&mut self) {
        loop {
            match self.events.try_recv() {
                Ok(event) => self.handle_event(event),
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                    self.push_event(format!("event stream lagged; skipped {skipped} events"));
                }
                Err(broadcast::error::TryRecvError::Closed) => {
                    self.push_event("event stream closed".to_string());
                    break;
                }
            }
        }
    }

    fn handle_event(&mut self, event: IkcEvent) {
        match &event {
            IkcEvent::Routed {
                request_id,
                target,
                intent,
                ..
            } => {
                self.push_event(format!("{request_id} routed to {target:?} as {intent}"));
            }
            IkcEvent::LogicExecuted {
                module, summary, ..
            } => {
                self.chat_log.push_back(ChatEntry {
                    role: "kernel_03",
                    text: format!("{module}: {summary}"),
                });
                if self.mode == DashboardMode::LivePreview {
                    self.preview_status = format!("Kernel_03 completed {module}");
                    self.preview_source = synthesize_preview_from_summary(summary);
                    clamp_string(&mut self.preview_source, MAX_PREVIEW_BYTES);
                }
            }
            IkcEvent::MediaFrameRendered {
                request_id,
                media_type,
                frame_index,
                total_frames,
                ..
            } => {
                self.media_frames.push_back(MediaFrameRecord {
                    request_id: request_id.clone(),
                    media_type: media_type.clone(),
                    frame_index: *frame_index,
                    total_frames: *total_frames,
                });
                while self.media_frames.len() > MAX_MEDIA_FRAMES {
                    self.media_frames.pop_front();
                }
            }
            IkcEvent::KnowledgeOrchestrated { reports, .. } => {
                self.push_event(format!("knowledge orchestration reports={}", reports.len()));
            }
            IkcEvent::MemoryPurged {
                dropped_fragments,
                remaining_fragments,
                ..
            } => {
                self.push_event(format!(
                    "Kernel_05 purged {dropped_fragments}; remaining={remaining_fragments}"
                ));
            }
            IkcEvent::Warning { message, .. } => self.push_event(format!("warning: {message}")),
            _ => self.push_event(format!("{event:?}")),
        }
        self.trim_memory();
    }

    fn push_event(&mut self, event: String) {
        self.event_log.push_back(event);
        while self.event_log.len() > MAX_EVENT_ENTRIES {
            self.event_log.pop_front();
        }
    }

    fn trim_memory(&mut self) {
        while self.chat_log.len() > MAX_CHAT_ENTRIES {
            self.chat_log.pop_front();
        }
        while self.event_log.len() > MAX_EVENT_ENTRIES {
            self.event_log.pop_front();
        }
        while self.media_frames.len() > MAX_MEDIA_FRAMES {
            self.media_frames.pop_front();
        }
        clamp_string(&mut self.preview_source, MAX_PREVIEW_BYTES);
        clamp_string(&mut self.chat_input, MAX_INPUT_BYTES);
        clamp_string(&mut self.preview_prompt, MAX_INPUT_BYTES);
        clamp_string(&mut self.media_prompt, MAX_INPUT_BYTES);

        while self.approx_heap_bytes() > DASHBOARD_MEMORY_BUDGET_BYTES {
            if self.media_frames.pop_front().is_some() {
                continue;
            }
            if self.event_log.pop_front().is_some() {
                continue;
            }
            if self.chat_log.pop_front().is_some() {
                continue;
            }
            self.preview_source.truncate(self.preview_source.len() / 2);
            break;
        }
    }

    fn approx_heap_bytes(&self) -> usize {
        let chat = self
            .chat_log
            .iter()
            .map(|entry| entry.text.len())
            .sum::<usize>();
        let events = self
            .event_log
            .iter()
            .map(|entry| entry.len())
            .sum::<usize>();
        let media = self
            .media_frames
            .iter()
            .map(|frame| std::mem::size_of::<MediaFrameRecord>() + frame.request_id.len())
            .sum::<usize>();
        chat + events
            + media
            + self.chat_input.len()
            + self.preview_prompt.len()
            + self.media_prompt.len()
            + self.preview_source.len()
            + self.preview_status.len()
    }

    fn draw_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("vortex_dashboard_header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Vortex Dashboard")
                        .strong()
                        .size(22.0)
                        .color(Color32::WHITE),
                );
                ui.label(
                    RichText::new("native egui canvas → Kernel_01 direct IKC")
                        .size(13.0)
                        .color(Color32::from_gray(170)),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let used = self.approx_heap_bytes();
                    let ratio = used as f32 / DASHBOARD_MEMORY_BUDGET_BYTES as f32;
                    ui.label(
                        RichText::new(format!(
                            "UI heap ≈ {:.2} MB / 50 MB",
                            used as f32 / (1024.0 * 1024.0)
                        ))
                        .color(if ratio < 0.75 {
                            Color32::LIGHT_GREEN
                        } else {
                            Color32::YELLOW
                        }),
                    );
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for mode in [
                    DashboardMode::IntelligentChatScenario,
                    DashboardMode::LivePreview,
                    DashboardMode::MediaProcessing,
                ] {
                    let selected = self.mode == mode;
                    let button = egui::Button::new(mode.short())
                        .fill(if selected {
                            mode.accent()
                        } else {
                            Color32::from_rgb(31, 34, 43)
                        })
                        .stroke(Stroke::new(1.0, mode.accent()))
                        .rounding(Rounding::same(10.0));
                    if ui.add(button).clicked() {
                        self.mode = mode;
                    }
                }
            });
            ui.add_space(6.0);
        });
    }

    fn draw_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("vortex_dashboard_side")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Kernel Matrix");
                ui.label("Kernel_01 receives every dashboard action directly through mpsc IKC.");
                ui.separator();
                ui.label(RichText::new("Active Mode").strong());
                ui.label(self.mode.title());
                ui.separator();
                ui.label(RichText::new("Recent IKC Events").strong());
                egui::ScrollArea::vertical()
                    .max_height(430.0)
                    .show(ui, |ui| {
                        for event in self.event_log.iter().rev().take(40) {
                            ui.label(
                                RichText::new(event)
                                    .size(11.0)
                                    .color(Color32::from_gray(185)),
                            );
                        }
                    });
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    if ui.button("Request memory purge via Kernel_01").clicked() {
                        self.send_to_kernel_01(
                            "purge inactive fragments and free ram via supervisor watchdog",
                        );
                    }
                });
            });
    }

    fn draw_dynamic_canvas(&mut self, ui: &mut egui::Ui) {
        let desired = Vec2::new(ui.available_width(), 132.0);
        let (rect, _response) = ui.allocate_exact_size(desired, Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, Rounding::same(18.0), self.mode.soft());

        let accent = self.mode.accent();
        let t = self.last_repaint.elapsed().as_secs_f32();
        for index in 0..9 {
            let phase = t * 0.7 + index as f32 * 0.73;
            let x = rect.left() + rect.width() * ((phase.sin() + 1.0) * 0.5);
            let y = rect.top() + 20.0 + (index as f32 * 13.0) % (rect.height() - 30.0);
            let radius = 18.0 + ((phase.cos() + 1.0) * 8.0);
            painter.circle_stroke(
                egui::pos2(x, y),
                radius,
                Stroke::new(
                    1.2,
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 110),
                ),
            );
        }

        painter.text(
            rect.left_top() + Vec2::new(22.0, 22.0),
            egui::Align2::LEFT_TOP,
            self.mode.title(),
            FontId::proportional(24.0),
            Color32::WHITE,
        );
        painter.text(
            rect.left_top() + Vec2::new(22.0, 58.0),
            egui::Align2::LEFT_TOP,
            match self.mode {
                DashboardMode::IntelligentChatScenario => {
                    "Scenario writing, reasoning, and module orchestration."
                }
                DashboardMode::LivePreview => {
                    "Generated apps, websites, and games render as bounded preview artifacts."
                }
                DashboardMode::MediaProcessing => {
                    "Image/audio/video work is tracked frame-by-frame without caching heavy pixels."
                }
            },
            FontId::proportional(14.0),
            Color32::from_gray(210),
        );
    }

    fn draw_chat_mode(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Prompt").strong());
            if ui.button("Scenario seed").clicked() {
                self.chat_input = "Write a strategic scenario using finance, humanities, and engineering modules.".to_string();
            }
        });
        ui.add(
            egui::TextEdit::multiline(&mut self.chat_input)
                .desired_rows(4)
                .hint_text("Ask Vortex Atoms AI..."),
        );
        if ui.button("Send to Kernel_01").clicked()
            || ui.input(|input| input.key_pressed(egui::Key::Enter) && input.modifiers.ctrl)
        {
            if !self.chat_input.trim().is_empty() {
                let text = self.chat_input.trim().to_string();
                self.chat_input.clear();
                self.send_to_kernel_01(text);
            }
        }
        ui.separator();
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for entry in &self.chat_log {
                    let color = match entry.role {
                        "user" => Color32::from_rgb(135, 206, 250),
                        "kernel_03" => Color32::from_rgb(160, 255, 190),
                        _ => Color32::from_gray(190),
                    };
                    ui.label(RichText::new(format!("{}: {}", entry.role, entry.text)).color(color));
                    ui.add_space(4.0);
                }
            });
    }

    fn draw_preview_mode(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Live Preview Request").strong());
        ui.add(
            egui::TextEdit::singleline(&mut self.preview_prompt)
                .hint_text("Describe the app, website, or game to generate..."),
        );
        if ui.button("Generate via Kernel_01").clicked() {
            let text = format!("live_preview_request: {}", self.preview_prompt.trim());
            self.preview_status = "Submitted preview request to Kernel_01".to_string();
            self.send_to_kernel_01(text);
        }
        ui.separator();
        ui.label(RichText::new(&self.preview_status).color(self.mode.accent()));
        egui::Frame::none()
            .fill(Color32::from_rgb(12, 14, 18))
            .stroke(Stroke::new(1.0, Color32::from_rgb(60, 66, 78)))
            .rounding(Rounding::same(14.0))
            .show(ui, |ui| {
                ui.set_min_height(360.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Preview Artifact")
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.label(
                        RichText::new("bounded text/structure view")
                            .size(11.0)
                            .color(Color32::GRAY),
                    );
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.monospace(&self.preview_source);
                });
            });
    }

    fn draw_media_mode(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Media Processing Request").strong());
        ui.add(
            egui::TextEdit::singleline(&mut self.media_prompt)
                .hint_text("Describe image, audio, or video generation..."),
        );
        ui.horizontal(|ui| {
            if ui.button("Image").clicked() {
                self.send_to_kernel_01(format!("render image: {}", self.media_prompt.trim()));
            }
            if ui.button("Audio").clicked() {
                self.send_to_kernel_01(format!("render audio: {}", self.media_prompt.trim()));
            }
            if ui.button("Video frames").clicked() {
                self.send_to_kernel_01(format!(
                    "render video frames: {}",
                    self.media_prompt.trim()
                ));
            }
        });
        ui.separator();
        ui.label(RichText::new("Frame Stream").strong());
        egui::ScrollArea::vertical().show(ui, |ui| {
            for frame in self.media_frames.iter().rev() {
                let progress = (frame.frame_index + 1) as f32 / frame.total_frames.max(1) as f32;
                ui.horizontal(|ui| {
                    ui.label(format!("{:?}", frame.media_type));
                    ui.add(egui::ProgressBar::new(progress).desired_width(240.0));
                    ui.label(format!(
                        "frame {}/{} · {}",
                        frame.frame_index + 1,
                        frame.total_frames,
                        frame.request_id
                    ));
                });
            }
        });
    }
}

impl eframe::App for VortexDashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_kernel_events();

        ctx.set_visuals(egui::Visuals::dark());
        self.draw_header(ctx);
        self.draw_side_panel(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_dynamic_canvas(ui);
            ui.add_space(12.0);
            match self.mode {
                DashboardMode::IntelligentChatScenario => self.draw_chat_mode(ui),
                DashboardMode::LivePreview => self.draw_preview_mode(ui),
                DashboardMode::MediaProcessing => self.draw_media_mode(ui),
            }
        });

        if self.last_repaint.elapsed() >= Duration::from_millis(33) {
            self.last_repaint = Instant::now();
            ctx.request_repaint();
        }
    }
}

impl Drop for VortexDashboardApp {
    fn drop(&mut self) {
        if let Some(matrix) = self.matrix.take() {
            self.runtime.block_on(matrix.shutdown());
        }
    }
}

fn clamp_string(value: &mut String, max_bytes: usize) {
    if value.len() <= max_bytes {
        return;
    }

    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    value.truncate(boundary);
}

fn default_preview_source() -> String {
    r#"<main class="vortex-app">
  <section class="hero">
    <h1>Vortex Atoms AI</h1>
    <p>Low-latency multimodal kernel matrix with mmap-backed knowledge.</p>
  </section>
  <canvas id="dynamic-canvas">bounded native preview</canvas>
</main>"#
        .to_string()
}

fn synthesize_preview_from_summary(summary: &str) -> String {
    format!(
        "// Kernel-generated preview summary\n// Memory bounded to {budget_mb} MB UI heap\n\n{summary}",
        budget_mb = DASHBOARD_MEMORY_BUDGET_BYTES / (1024 * 1024)
    )
}
