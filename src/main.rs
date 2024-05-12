use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::{run_native, App};
use egui::Ui;
use witness_tas::communication::{client_thread, ControllerToTasMessage, TasToControllerMessage};
use witness_tas::tas_player::{PlaybackState, TraceDrawOption};

#[derive(PartialEq)]
enum TasInterfaceTab {
    Playback,
    Trace,
    Config,
    About,
}

struct TasInterface {
    // Communication with the tas player
    to_server: Sender<ControllerToTasMessage>,
    from_server: Receiver<TasToControllerMessage>,

    // Tas controls
    filename: String,
    playback_state: PlaybackState,
    skipto: u32,
    looping: bool,

    // Info
    player_pos: (f32, f32, f32), // Replace with vec3
    player_ang: (f32, f32),      // Replace with vec2
    current_tick: u32,
    parse_errors: Vec<String>,

    // Trace
    trace_selected_tick: u32,
    trace_continuous_teleport: bool,
    trace_display_opts: TraceDrawOption,

    // Interface state
    current_tab: TasInterfaceTab,
}

impl TasInterface {
    fn new() -> Self {
        let (send, from_server) = channel();
        let (to_server, recv) = channel();
        std::thread::spawn(|| client_thread(send, recv));

        TasInterface {
            to_server,
            from_server,
            filename: "example.wtas".to_string(),
            playback_state: PlaybackState::Stopped,
            skipto: 0,
            looping: false,
            player_pos: (0., 0., 0.),
            player_ang: (0., 0.),
            current_tick: 0,
            parse_errors: vec![],
            trace_selected_tick: 0,
            trace_continuous_teleport: false,
            trace_display_opts: Default::default(),
            current_tab: TasInterfaceTab::Playback,
        }
    }

    fn connect(&mut self) {
        let (send, from_server) = channel();
        let (to_server, recv) = channel();
        std::thread::spawn(|| client_thread(send, recv));

        self.to_server = to_server;
        self.from_server = from_server;

        // TODO: resend info to server
        self.to_server
            .send(ControllerToTasMessage::SkipTo(self.skipto))
            .unwrap();
    }

    fn update_from_server(&mut self) {
        for msg in self.from_server.try_iter() {
            match msg {
                TasToControllerMessage::PlaybackState(state) => {
                    self.playback_state = state;
                    // If the script starts playing, it means there are no errors
                    if state == PlaybackState::Playing || state == PlaybackState::Skipping {
                        self.parse_errors.clear()
                    }
                }
                TasToControllerMessage::CurrentTick(tick) => self.current_tick = tick,
                TasToControllerMessage::ParseErrors(errors) => self.parse_errors = errors,
                TasToControllerMessage::CarlInfo { pos, ang } => {
                    self.player_pos = pos;
                    self.player_ang = ang;
                }
            }
        }
    }
}

impl App for TasInterface {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_from_server();

        // App
        egui::CentralPanel::default().show(ctx, |ui| {
            // Try to send a no-op to see if channel is alive
            if self
                .to_server
                .send(ControllerToTasMessage::SkipTo(self.skipto))
                .is_err()
            {
                if ui.button("Connect").clicked() {
                    self.connect();
                }
                return;
            }

            // Info is always there
            self.info(ui);
            ui.add(egui::Separator::default().grow(10.));

            // Draw the tabs
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, TasInterfaceTab::Playback, "Playback");
                ui.selectable_value(&mut self.current_tab, TasInterfaceTab::Trace, "Trace");
                ui.selectable_value(&mut self.current_tab, TasInterfaceTab::Config, "Config");
                ui.selectable_value(&mut self.current_tab, TasInterfaceTab::About, "About");
            });

            ui.separator();

            // Draw the selected tab
            match self.current_tab {
                TasInterfaceTab::Playback => self.playback_controls_tab(ui),
                TasInterfaceTab::Trace => self.trace_controls_tab(ui),
                TasInterfaceTab::Config => self.config_tab(ui),
                TasInterfaceTab::About => self.about_tab(ui),
            }
        });

        ctx.request_repaint();
    }
}

impl TasInterface {
    /// Draw the info section
    fn info(&mut self, ui: &mut Ui) {
        ui.heading("Info");
        ui.label(format!(
            "pos: {:4.3} {:4.3} {:4.3}",
            self.player_pos.0, self.player_pos.1, self.player_pos.2
        ));
        ui.label(format!(
            "ang: {:1.3} {:1.3}",
            self.player_ang.0, self.player_ang.1
        ));
        ui.label(format!("Current tick: {}", self.current_tick));

        if !self.parse_errors.is_empty() {
            ui.heading("Parse errors");
            for error in &self.parse_errors {
                ui.label(error);
            }
        }
    }

    /// Draw the playback controls
    fn playback_controls_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let label = ui.label("TAS file");
            ui.text_edit_singleline(&mut self.filename)
                .labelled_by(label.id)
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.looping, "Loop TAS");

            if !self.looping {
                let play_button_enabled = self.playback_state == PlaybackState::Stopped
                    || self.playback_state == PlaybackState::Paused;
                if ui
                    .add_enabled(play_button_enabled, egui::Button::new("Play"))
                    .clicked()
                {
                    self.to_server
                        .send(ControllerToTasMessage::PlayFile(self.filename.clone()))
                        .unwrap();
                }

                let stop_button_enabled = self.playback_state != PlaybackState::Stopped;
                if ui
                    .add_enabled(stop_button_enabled, egui::Button::new("Stop"))
                    .clicked()
                {
                    self.to_server.send(ControllerToTasMessage::Stop).unwrap();
                }

                let frame_by_frame_button_enabled = self.playback_state != PlaybackState::Stopped;
                let frame_button = if self.playback_state == PlaybackState::Paused {
                    ui.add_enabled(
                        frame_by_frame_button_enabled,
                        egui::Button::new("Next Frame"),
                    )
                } else {
                    ui.add_enabled(frame_by_frame_button_enabled, egui::Button::new("Pause"))
                };

                if frame_button.clicked() {
                    self.to_server
                        .send(ControllerToTasMessage::AdvanceFrame)
                        .unwrap();
                }
            }
        });

        ui.horizontal(|ui| {
            let skip_label = ui.label("Skip to tick: ");
            let skipto = ui
                .add(egui::DragValue::new(&mut self.skipto))
                .labelled_by(skip_label.id);

            if skipto.changed() {
                match self
                    .to_server
                    .send(ControllerToTasMessage::SkipTo(self.skipto))
                {
                    Ok(_) => {}
                    Err(err) => println!("{err}"),
                }
            }
        });

        if self.looping
            && self.playback_state != PlaybackState::Playing
            && self.playback_state != PlaybackState::Skipping
        {
            self.to_server
                .send(ControllerToTasMessage::PlayFile(self.filename.clone()))
                .unwrap();
        }
    }

    /// Draw the trace controls
    fn trace_controls_tab(&mut self, ui: &mut Ui) {
        ui.heading("Teleport");

        ui.horizontal(|ui| {
            let label = ui.label("Selected tick: ");
            let dragvalue = ui
                .add(egui::DragValue::new(&mut self.trace_selected_tick))
                .labelled_by(label.id);

            if ui
                .add_enabled(
                    !self.trace_continuous_teleport,
                    egui::Button::new("Teleport"),
                )
                .clicked()
            {
                self.to_server
                    .send(ControllerToTasMessage::TeleportToTick(
                        self.trace_selected_tick,
                    ))
                    .unwrap();
            }

            ui.checkbox(&mut self.trace_continuous_teleport, "Continuous");

            // Teleport without completely spamming
            if self.trace_continuous_teleport && dragvalue.changed() {
                self.to_server
                    .send(ControllerToTasMessage::TeleportToTick(
                        self.trace_selected_tick,
                    ))
                    .unwrap();
            }
        });

        ui.heading("Display");
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("")
                .selected_text(format!("{}", self.trace_display_opts.variant_name_simple()))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.trace_display_opts,
                        TraceDrawOption::First(0),
                        "First",
                    );
                    ui.selectable_value(
                        &mut self.trace_display_opts,
                        TraceDrawOption::Last(0),
                        "Last",
                    );
                    ui.selectable_value(
                        &mut self.trace_display_opts,
                        TraceDrawOption::Between(0, 0),
                        "Between",
                    );
                });

            let mut trace_opt_changed = false;
            match self.trace_display_opts {
                TraceDrawOption::First(mut from_start) => {
                    if ui.add(egui::DragValue::new(&mut from_start)).changed() {
                        self.trace_display_opts = TraceDrawOption::First(from_start);
                        trace_opt_changed = true;
                    }
                }
                TraceDrawOption::Last(mut from_end) => {
                    if ui.add(egui::DragValue::new(&mut from_end)).changed() {
                        self.trace_display_opts = TraceDrawOption::Last(from_end);
                        trace_opt_changed = true;
                    }
                }
                TraceDrawOption::Between(mut start, mut end) => {
                    if ui.add(egui::DragValue::new(&mut start)).changed() {
                        self.trace_display_opts = TraceDrawOption::Between(start, end);
                        trace_opt_changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut end)).changed() || end < start {
                        self.trace_display_opts = TraceDrawOption::Between(start, end.max(start));
                        trace_opt_changed = true;
                    }
                }
            };
            ui.label("ticks");

            if trace_opt_changed {
                self.to_server.send(ControllerToTasMessage::TraceOptions(self.trace_display_opts)).unwrap();
            }
        });
    }
    fn config_tab(&mut self, ui: &mut Ui) {
        ui.label("Under Construction");
    }
    fn about_tab(&mut self, ui: &mut Ui) {
        ui.label("Under Construction");
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let app = TasInterface::new();

    run_native("rainbow's TAS tool", options, Box::new(|_cc| Box::new(app)))?;

    println!("bye");
    Ok(())
}
