use std::sync::mpsc::{channel, Receiver, Sender};
use std::{collections::VecDeque, sync::Mutex};

use crate::communication::{server_thread, ControllerToTasMessage, TasToControllerMessage};
use crate::hooks::CAMERA_ANG;
use crate::{
    hooks::{DoRestart, MAIN_LOOP_COUNT, NEW_GAME_FLAG, PLAYER},
    script::{self, Script, StartType},
    witness::witness_types::Vec3,
};
use chumsky::Parser as _;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

pub static mut TAS_PLAYER: Mutex<Option<TasPlayer>> = Mutex::new(None);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,

    // This state is only used for the tas controller interface
    Skipping,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HalfControllerState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub running: bool,

    pub mouse_pos: (i32, i32),
    pub left_click: bool,
    pub right_click: bool,
}

#[derive(Debug, Default)]
pub struct ControllerState {
    pub current: HalfControllerState,
    pub previous: HalfControllerState,
}

pub struct TasPlayer {
    send: Sender<TasToControllerMessage>,
    recv: Receiver<ControllerToTasMessage>,
    state: PlaybackState,

    start_tick: u32,
    current_tick: u32,
    skipto_tick: u32,

    next_line: usize,
    script_name: String,
    script: Option<script::Script>,

    controller: ControllerState,

    // Utilities
    pub trace: Playertrace,
}

impl TasPlayer {
    /// Inits the global
    pub fn init() {
        let mut tas_player = unsafe { TAS_PLAYER.lock().unwrap() };

        if tas_player.is_none() {
            *tas_player = Some(Self::new());
        }
    }

    /// Creates a new TasPlayer
    fn new() -> Self {
        let (send, from_client) = channel();
        let (to_client, recv) = channel();
        std::thread::spawn(|| server_thread(send, recv));

        Self {
            send: to_client,
            recv: from_client,
            state: PlaybackState::Stopped,
            start_tick: 0,
            current_tick: 0,
            skipto_tick: 0,
            next_line: 0,
            script_name: "".to_string(),
            script: None,
            controller: Default::default(),
            trace: Default::default(),
        }
    }

    /// Starts the TAS
    /// If file is None, replay the current tas
    pub fn start(&mut self, file: Option<String>) {
        self.stop();

        if let Some(file) = file {
            self.script_name = file;
        }

        self.script = match std::fs::read_to_string("./tas/".to_string() + &self.script_name) {
            Err(err) => {
                error!("{err}");
                self.send
                    .send(TasToControllerMessage::ParseErrors(vec![err.to_string()]))
                    .unwrap();
                None
            }
            Ok(src) => match Script::get_parser().parse(src) {
                Err(parse_errs) => {
                    for err in &parse_errs {
                        error!("Parse error: {err}");
                    }
                    self.send
                        .send(TasToControllerMessage::ParseErrors(
                            parse_errs.iter().map(|e| format!("{e}")).collect(),
                        ))
                        .unwrap();
                    None
                }
                Ok(mut script) => match script.pre_process() {
                        Ok(_) => Some(script),
                        Err(err) => {
                            error!("Parse error: {err}");
                        self.send
                            .send(TasToControllerMessage::ParseErrors(vec![err]))
                            .unwrap();
                            None
                    }
                },
            },
        };

        let Some(script) = &self.script else { return };

        match script.start {
            StartType::Now => {}
            StartType::NewGame => unsafe {
                // These actions are lifted from the function in the witness
                //  that handle the menu for new game
                NEW_GAME_FLAG.write(true);
                DoRestart.call();
            },
            StartType::Save(_) => {
                error!("Starting from a save is not implemented yet. Starting now instead.")
            }
        }

        self.controller = Default::default();
        self.start_tick = unsafe { MAIN_LOOP_COUNT.read() };
        self.current_tick = 0;
        self.next_line = 0;
        self.state = PlaybackState::Playing;
        // self.skipto_tick = script.skipto;

        self.trace.clear();

        info!("Started TAS")
    }

    /// Stops the TAS
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;

        let ticks = self.current_tick;
        info!("Stopped TAS after {ticks} ticks.")
    }

    /// Get the controller input and possibly advance state.
    pub fn get_controller(&mut self) -> Option<&ControllerState> {
        self.update_from_server();

        // Update the controller
        let pos = unsafe { PLAYER.read().position };
        let ang = unsafe { CAMERA_ANG.read() };
        self.send
            .send(TasToControllerMessage::CarlInfo {
                pos: (pos.x, pos.y, pos.z),
                ang: (ang.x, ang.y),
            })
            .unwrap();

        self.send
            .send(TasToControllerMessage::PlaybackState(
                self.get_playback_state(),
            ))
            .unwrap();

        // If we are not running, exit
        if self.state == PlaybackState::Stopped {
            return None;
        }

        let script = self.script.as_ref()?;

        if self.next_line >= script.lines.len() {
            self.stop();
            return None;
        }

        // Update controller
        // Get pressed keys
        let current_tick = unsafe { MAIN_LOOP_COUNT.read() } - self.start_tick;
        if self.current_tick != current_tick {
            self.send
                .send(TasToControllerMessage::CurrentTick(current_tick))
                .unwrap();

            // Update the player pos history
            self.trace.push(unsafe { PLAYER.read().position });

            self.current_tick = current_tick;
            let next_line = &script.lines[self.next_line];

            self.controller.previous = self.controller.current;

            // Do the auto lifting of the mouse buttons
            if self.controller.previous.left_click {
                self.controller.current.left_click = false;
            }
            if self.controller.previous.right_click {
                self.controller.current.right_click = false;
            }

            if next_line.tick == current_tick {
                self.next_line += 1;

                for key in &next_line.keys {
                    match key {
                        // Movement
                        'U' => self.controller.current.forward = true,
                        'u' => self.controller.current.forward = false,
                        'L' => self.controller.current.left = true,
                        'l' => self.controller.current.left = false,
                        'D' => self.controller.current.backward = true,
                        'd' => self.controller.current.backward = false,
                        'R' => self.controller.current.right = true,
                        'r' => self.controller.current.right = false,

                        // Sprint
                        'S' => self.controller.current.running = true,
                        's' => self.controller.current.running = false,

                        // Toggle puzzle
                        'P' => self.controller.current.left_click = true,
                        'p' => self.controller.current.right_click = true,

                        _ => {}
                    }
                }

                if let Some(mouse) = next_line.mouse {
                    self.controller.current.mouse_pos = mouse;
                }
            }
        }

        // Return it
        Some(&self.controller)
    }

    fn update_from_server(&mut self) {
        let Ok(msg) = self.recv.try_recv() else {
            return;
        };

        match msg {
            ControllerToTasMessage::PlayFile(filename) => self.start(Some(filename)),
            ControllerToTasMessage::Stop => self.stop(),
            ControllerToTasMessage::SkipTo(tick) => self.skipto_tick = tick,
            ControllerToTasMessage::AdvanceFrame => error!("Frame by frame is not implemented yet"),
        }
    }

    pub fn should_do_skipping(&self) -> bool {
        // Only skip after 60 frames, the "eyes opening" animation fucks things up
        self.state == PlaybackState::Playing
            && self.current_tick < self.skipto_tick
            && self.current_tick > 60
    }

    pub fn get_playback_state(&self) -> PlaybackState {
        if self.state == PlaybackState::Playing && self.current_tick < self.skipto_tick {
            PlaybackState::Skipping
        } else {
            self.state
        }
    }

    pub fn get_current_tick(&self) -> u32 {
        self.current_tick
    }
}

#[derive(Default)]
pub struct Playertrace {
    pub positions: Vec<Vec3>,
}

impl Playertrace {
    pub fn clear(&mut self) {
        self.positions.clear()
    }
}
