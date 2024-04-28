use std::sync::Mutex;

use crate::{hooks::MAIN_LOOP_COUNT, script};
use chumsky::Parser as _;
use tracing::{debug, error};

pub static mut TAS_PLAYER: Mutex<Option<TasPlayer>> = Mutex::new(None);

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
    playing: bool,
    start_tick: u32,
    current_tick: u32,
    next_line: usize,
    script: script::Script,

    controller: ControllerState,
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
                    // debug!("{script:#?}");
                    Some(Self {
                        playing: false,
                        start_tick: 0,
                        current_tick: 0,
                        next_line: 0,
                        script,
                        controller: Default::default(),
                    })
                }
            },
        }
    }

    /// Starts the TAS
    pub fn start(&mut self) {
        self.start_tick = unsafe { MAIN_LOOP_COUNT.read() };
        self.current_tick = 0;
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
        let current_tick = unsafe { MAIN_LOOP_COUNT.read() } - self.start_tick;
        if self.current_tick != current_tick {
            self.current_tick = current_tick;
            let next_line = &self.script.lines[self.next_line];

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
}
