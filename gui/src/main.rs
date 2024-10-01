use std::sync::mpsc::{channel, Receiver, Sender};

use eframe::{run_native, App};
use egui::Ui;
use common::communication::{client_thread, ControllerToTasMessage, TasToControllerMessage};
use common::tas::{PlaybackState, TraceDrawOptions, TraceInterval};

mod platform;
use platform::try_inject;

mod widgets;
use widgets::scrollable_dragvalue;

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
    looping: bool,
    skipto: u32,
    pauseat: u32,
    always_pause_after_skip: bool,

    // Info
    player_pos: (f32, f32, f32), // Replace with vec3
    player_ang: (f32, f32),      // Replace with vec2
    current_tick: u32,
    latest_puzzle_unlock: u32,
    parse_errors: Vec<String>,

    // Trace
    trace_selected_tick: u32,
    trace_continuous_teleport: bool,
    trace_display_opts: TraceDrawOptions,

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
            looping: false,
            skipto: 0,
            pauseat: 0,
            always_pause_after_skip: false,
            player_pos: (0., 0., 0.),
            player_ang: (0., 0.),
            current_tick: 0,
            latest_puzzle_unlock: 0,
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
                TasToControllerMessage::PuzzleUnlock(tick) => self.latest_puzzle_unlock = tick,
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
                ui.label(
                    "Failed to connect to The Witness! Open the game and press the button below.",
                );
                if ui.button("Inject & Connect").clicked() {
                    try_inject();
                    self.connect();
                }
                ui.separator();
                self.about_tab(ui);
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
        let pos = ui
            .label(format!(
                "pos: {:4.3} {:4.3} {:4.3}",
                self.player_pos.0, self.player_pos.1, self.player_pos.2
            ))
            .on_hover_text("Click to put exact position into clipboard");
        let ang = ui
            .label(format!(
                "ang: {:1.3} {:1.3}",
                self.player_ang.0, self.player_ang.1
            ))
            .on_hover_text("Click to put exact angles into clipboard");
        ui.label(format!("Current tick: {}", self.current_tick));

        ui.label(format!(
            "Latest puzzle unlock: {}",
            self.latest_puzzle_unlock
        ));

        if !self.parse_errors.is_empty() {
            ui.heading("Parse errors");
            for error in &self.parse_errors {
                ui.label(error);
            }
        }

        // Paste exact pos/ang into clipboard on click
        if pos.clicked() {
            ui.output_mut(|o| {
                o.copied_text = format!(
                    "{:10.15} {:10.15} {:10.15}",
                    self.player_pos.0, self.player_pos.1, self.player_pos.2
                )
            })
        }
        if ang.clicked() {
            ui.output_mut(|o| {
                o.copied_text = format!("{:1.15} {:1.15}", self.player_ang.0, self.player_ang.1)
            })
        }
    }

    /// Draw the playback controls
    fn playback_controls_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.looping, "Loop mode");

            if !self.looping {
                ui.separator();
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
                .add(scrollable_dragvalue(&mut self.skipto))
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

        ui.horizontal(|ui| {
            let pauseat_label = ui.label("Pause at tick: ");
            let pauseat = ui
                .add_enabled(
                    !self.always_pause_after_skip,
                    scrollable_dragvalue(&mut self.pauseat),
                )
                .labelled_by(pauseat_label.id);

            if self.always_pause_after_skip {
                self.pauseat = self.skipto + 1;
            }

            if pauseat.changed() || self.always_pause_after_skip {
                match self
                    .to_server
                    .send(ControllerToTasMessage::PauseAt(self.pauseat))
                {
                    Ok(_) => {}
                    Err(err) => println!("{err}"),
                }
            }
        });

        ui.checkbox(&mut self.always_pause_after_skip, "Pause after skip");

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                let label = ui.label("File:");
                ui.text_edit_singleline(&mut self.filename)
                    .labelled_by(label.id)
            });
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
                .add(scrollable_dragvalue(&mut self.trace_selected_tick))
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
                .selected_text(format!(
                    "{}",
                    self.trace_display_opts.interval.variant_name_simple()
                ))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.trace_display_opts.interval,
                        TraceInterval::First(0),
                        "First",
                    );
                    ui.selectable_value(
                        &mut self.trace_display_opts.interval,
                        TraceInterval::Last(0),
                        "Last",
                    );
                    ui.selectable_value(
                        &mut self.trace_display_opts.interval,
                        TraceInterval::Between(0, 0),
                        "Between",
                    );
                });

            let mut trace_opt_changed = false;
            match self.trace_display_opts.interval {
                TraceInterval::First(mut from_start) => {
                    if ui.add(scrollable_dragvalue(&mut from_start)).changed() {
                        self.trace_display_opts.interval = TraceInterval::First(from_start);
                        trace_opt_changed = true;
                    }
                }
                TraceInterval::Last(mut from_end) => {
                    if ui.add(scrollable_dragvalue(&mut from_end)).changed() {
                        self.trace_display_opts.interval = TraceInterval::Last(from_end);
                        trace_opt_changed = true;
                    }
                }
                TraceInterval::Between(mut start, mut end) => {
                    if ui.add(scrollable_dragvalue(&mut start)).changed() {
                        self.trace_display_opts.interval = TraceInterval::Between(start, end);
                        trace_opt_changed = true;
                    }
                    if ui.add(scrollable_dragvalue(&mut end)).changed() || end < start {
                        self.trace_display_opts.interval =
                            TraceInterval::Between(start, end.max(start));
                        trace_opt_changed = true;
                    }
                }
            };
            ui.label("ticks");

            if trace_opt_changed {
                self.to_server
                    .send(ControllerToTasMessage::TraceOptions(
                        self.trace_display_opts,
                    ))
                    .unwrap();
            }
        });

        let radius = ui.horizontal(|ui| {
            ui.label("Trace sphere radius:");
            ui.add(
                egui::DragValue::new(&mut self.trace_display_opts.sphere_radius)
                    .clamp_range(0.005..=0.08)
                    .speed(0.01),
            )
        });
        let z_off = ui.horizontal(|ui| {
            ui.label("Trace vertical offset:");
            ui.add(
                egui::DragValue::new(&mut self.trace_display_opts.z_offset)
                    .clamp_range(-1.0..=1.0)
                    .speed(0.1),
            )
        });
        let puzzle_click_dist = ui.horizontal(|ui| {
            ui.label("Click indicator distance:");
            ui.add(
                egui::DragValue::new(
                    &mut self
                        .trace_display_opts
                        .puzzle_click_indicator_distance_multiplier,
                )
                .clamp_range(1.0..=1000.0)
                .speed(0.1),
            )
        });
        let puzzle_click_radius = ui.horizontal(|ui| {
            ui.label("Click indicator radius:");
            ui.add(
                egui::DragValue::new(&mut self.trace_display_opts.puzzle_click_indicator_radius)
                    .clamp_range(0.005..=0.08)
                    .speed(0.01),
            )
        });

        let reset_defaults = ui.button("Reset defaults").clicked();
        if reset_defaults {
            self.trace_display_opts = Default::default();
        }

        if radius.inner.changed()
            || z_off.inner.changed()
            || puzzle_click_dist.inner.changed()
            || puzzle_click_radius.inner.changed()
            || reset_defaults
        {
            self.to_server
                .send(ControllerToTasMessage::TraceOptions(
                    self.trace_display_opts,
                ))
                .unwrap();
        }
    }

    /// Draw the config TAB
    fn config_tab(&mut self, ui: &mut Ui) {
        // TODO before this: refactor messages and server/client state of the other
        ui.label("Under Construction");
    }

    /// Draw the about TAB
    fn about_tab(&mut self, ui: &mut Ui) {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.label("This TAS tool for The Witness was made by RainbowwPhoenixx");
            ui.hyperlink_to(
                "Source code on github",
                "https://github.com/RainbowwPhoenixx/witas",
            );

            ui.label(format!(
                "version: v{}\ncommit: {}",
                env!("CARGO_PKG_VERSION"),
                env!("GIT_HASH")
            ));
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 340.0]),
        follow_system_theme: false,
        ..Default::default()
    };

    let app = TasInterface::new();

    run_native("rainbow's TAS tool", options, Box::new(|_cc| Box::new(app)))?;

    println!("bye");
    Ok(())
}
