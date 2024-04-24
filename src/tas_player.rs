use std::sync::Mutex;

use crate::script;
use chumsky::Parser as _;
use tracing::{error, debug};


pub static mut TAS_PLAYER: Mutex<Option<TasPlayer>> = Mutex::new(None);

pub struct ControllerState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
        }
    }
}

pub struct TasPlayer {
    playing: bool,
    start_tick: u64,
    next_line: usize,
    script: script::Script,

    controller: ControllerState,

    // This should be removed later, in favor of making
    // the "update_controller" function read current tick
    // from global state
    current_tick: u64,
}

impl TasPlayer {
    /// Creates a TasPlayer from a file
    pub fn from_file(s: String) -> Option<Self> {
        match std::fs::read_to_string(s) {
            Err(err) => {
                error!("{err}");
                None
            }
            Ok(src) => match script::parser().parse(src) {
                Err(parse_errs) => {
                    parse_errs
                        .into_iter()
                        .for_each(|e| error!("Parse error: {}", e));
                    None
                }
                Ok(script) => {
                    debug!("{script:#?}");
                    Some(Self {
                        playing: false,
                        start_tick: 0,
                        next_line: 0,
                        script,
                        controller: Default::default(),
                        current_tick: 0,
                    })
                }
            },
        }
    }

    /// Starts the TAS
    pub fn start(&mut self, start_tick: u64) {
        self.start_tick = start_tick;
        self.current_tick = start_tick;
        self.playing = true;
    }

    /// Stops the TAS
    pub fn stop(&mut self) {
        self.playing = false;
    }

    /// Get the controller input and possibly advance state.
    pub fn get_controller(&mut self) -> Option<&ControllerState> {
        // If we are not running, exit
        if !self.playing {
            return None;
        }

        if self.next_line >= self.script.lines.len() {
            self.stop();
            return None;
        }

        // Update controller
        // Get pressed keys
        self.current_tick += 1;
        let next_line = &self.script.lines[self.next_line];
        if next_line.tick == self.current_tick {
            self.next_line += 1;

            for key in &next_line.keys {
                match key {
                    'Z' => self.controller.forward = true,
                    'z' => self.controller.forward = false,
                    'Q' => self.controller.left = true,
                    'q' => self.controller.left = false,
                    'S' => self.controller.backward = true,
                    's' => self.controller.backward = false,
                    'D' => self.controller.right = true,
                    'd' => self.controller.right = false,
                    _ => {}
                }
            }
        }

        // Return it
        Some(&self.controller)
    }
}
