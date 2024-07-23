use std::collections::HashMap;
use std::ffi::CStr;
use std::ops::Range;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

use crate::communication::{server_thread, ControllerToTasMessage, TasToControllerMessage};
use crate::hooks::{
    CopyString, APPDATA_PATH, INTERACTION_STATUS, PLAYER_ANG, PLAYER_POS, SAVE_PATH,
};
use crate::witness::witness_types::{InteractionStatus, Vec2};
use crate::{
    hooks::{DoRestart, LOAD_SAVE_FLAG, MAIN_LOOP_COUNT, NEW_GAME_FLAG, PLAYER},
    script::{self, Script, StartType},
    witness::witness_types::Vec3,
};
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
    pauseat_tick: u32,

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
            pauseat_tick: 0,
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
            Ok(src) => match Script::try_from(src) {
                Err(parse_errs) => {
                    for err in &parse_errs {
                        error!("Parse error: {err}");
                    }
                    self.send
                        .send(TasToControllerMessage::ParseErrors(parse_errs))
                        .unwrap();
                    None
                }
                Ok(script) => Some(script),
            },
        };

        let Some(script) = &self.script else { return };

        match &script.start {
            StartType::Now => {}
            StartType::NewGame => unsafe {
                // These actions are lifted from the function in the witness
                // that handle the menu for new game
                NEW_GAME_FLAG.write(true);
                DoRestart.call();
            },
            StartType::Save(path) => unsafe {
                let str_ptr = APPDATA_PATH.read();

                if str_ptr == std::ptr::null_mut() {
                    error!("Unable to find save folder, nullptr");
                    return;
                }

                let appdata_location = CStr::from_ptr(str_ptr).to_string_lossy().to_string();
                let full_path = format!("{}\\{}", appdata_location, path);

                info!("Loading save {full_path}");

                let Ok(c_str) = std::ffi::CString::new(full_path.as_bytes()) else {
                    error!("Invalid filename: {full_path}");
                    return;
                };

                // We use the game's copy string so that the game allocates
                // the string for us, and does not try to free memory allocated
                // by us (explicitely discouraged by into_raw's documentation)
                SAVE_PATH.write(CopyString.call(c_str.into_raw()));
                LOAD_SAVE_FLAG.write(true);
                DoRestart.call();
            },
        }

        self.controller = Default::default();
        self.start_tick = unsafe { MAIN_LOOP_COUNT.read() };
        self.current_tick = 0;
        self.next_line = 0;
        self.state = PlaybackState::Playing;

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
        self.update_from_server(false);

        // Update the controller
        let pos = unsafe { PLAYER.read().position };
        let ang = unsafe { PLAYER_ANG.read() };
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

            if current_tick == self.pauseat_tick {
                self.state = PlaybackState::Paused;

                self.send
                    .send(TasToControllerMessage::PlaybackState(
                        self.get_playback_state(),
                    ))
                    .unwrap();
            }

            // Update the player pos history
            unsafe {
                self.trace.push(
                    PLAYER.read().position,
                    PLAYER_ANG.read(),
                    INTERACTION_STATUS.read().try_into().unwrap(),
                )
            };

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

    fn update_from_server(&mut self, block: bool) {
        // We're using a loop and not try_iter here because the borrow checker
        // doesn't like it
        loop {
            let msg = if block && self.state == PlaybackState::Paused {
                self.recv.recv().ok()
            } else {
                self.recv.try_recv().ok()
            };

            let msg = match msg {
                Some(msg) => msg,
                None => return,
            };

            match msg {
                ControllerToTasMessage::PlayFile(filename) => {
                    if self.state == PlaybackState::Paused {
                        self.state = PlaybackState::Playing;
                    } else {
                        self.start(Some(filename));
                    }
                }
                ControllerToTasMessage::Stop => self.stop(),
                ControllerToTasMessage::SkipTo(tick) => self.skipto_tick = tick,
                ControllerToTasMessage::PauseAt(tick) => self.pauseat_tick = tick,
                ControllerToTasMessage::AdvanceFrame => {
                    self.state = PlaybackState::Paused;
                    return;
                }
                ControllerToTasMessage::TeleportToTick(tick) => {
                    if self.state == PlaybackState::Stopped {
                        self.trace.teleport_tick(tick);
                    }
                }
                ControllerToTasMessage::TraceOptions(opt) => self.trace.draw_option = opt,
            }
        }
    }

    pub fn block_until_next_frame(&mut self) {
        self.update_from_server(true)
    }

    pub fn add_puzzle_click(&mut self, cam_pos: Vec3, click_dir: Vec3) {
        let current_tick = unsafe { MAIN_LOOP_COUNT.read() } - self.start_tick - 1;
        self.trace
            .add_puzzle_click(current_tick, cam_pos, click_dir)
    }

    pub fn send_puzzle_unlock(&self) {
        let current_tick = unsafe { MAIN_LOOP_COUNT.read() } - self.start_tick;
        self.send
            .send(TasToControllerMessage::PuzzleUnlock(current_tick))
            .unwrap();
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TraceInterval {
    First(u32),
    Last(u32),
    Between(u32, u32),
}

impl TraceInterval {
    pub fn variant_name_simple(&self) -> &str {
        match self {
            TraceInterval::First(_) => "First",
            TraceInterval::Last(_) => "Last",
            TraceInterval::Between(_, _) => "Between",
        }
    }
}

impl Default for TraceInterval {
    fn default() -> Self {
        Self::Last(100)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct TraceDrawOptions {
    pub sphere_radius: f32,
    pub z_offset: f32,
    pub puzzle_click_indicator_distance_multiplier: f32,
    pub puzzle_click_indicator_radius: f32,
    pub interval: TraceInterval,
}

impl Default for TraceDrawOptions {
    fn default() -> Self {
        Self {
            sphere_radius: 0.05,
            z_offset: Default::default(),
            puzzle_click_indicator_distance_multiplier: Default::default(),
            puzzle_click_indicator_radius: 0.01,
            interval: Default::default(),
        }
    }
}

pub struct TraceTick {
    pub pos: Vec3,
    pub ang: Vec2,
    pub interact: InteractionStatus,
}

#[derive(Default)]
pub struct Playertrace {
    pub draw_option: TraceDrawOptions,
    ticks: Vec<TraceTick>,
    puzzle_clicks: HashMap<u32, (Vec3, Vec3)>,
}

impl Playertrace {
    pub fn clear(&mut self) {
        self.ticks.clear();
        self.puzzle_clicks.clear();
    }

    /// Add a point to the trace
    pub fn push(&mut self, pos: Vec3, ang: Vec2, interact: InteractionStatus) {
        self.ticks.push(TraceTick { pos, ang, interact })
    }

    /// Add a puzzle click debug
    pub fn add_puzzle_click(&mut self, tick: u32, cam_pos: Vec3, click_dir: Vec3) {
        self.puzzle_clicks.insert(tick, (cam_pos, click_dir));
    }

    /// Return the list of positions to display in-world
    pub fn get_pos_to_show(&self) -> &[TraceTick] {
        let range = self.get_interval();

        if range.len() == 0 {
            return &[];
        }
        &self.ticks[range]
    }

    pub fn get_puzzle_clicks(&self) -> Vec<(Vec3, Vec3)> {
        let range = self.get_interval();
        self.puzzle_clicks
            .iter()
            .filter_map(|(&tick, &info)| {
                if range.contains(&(tick as usize)) {
                    Some(info)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Teleport player to the given tick
    pub fn teleport_tick(&self, tick: u32) -> Option<()> {
        let tick = self.ticks.get(tick as usize)?;

        unsafe {
            PLAYER_POS.write(tick.pos);
            PLAYER_ANG.write(tick.ang);
        };

        Some(())
    }

    pub fn get_interval(&self) -> Range<usize> {
        let (start, end) = match self.draw_option.interval {
            TraceInterval::First(from_start) => (0, from_start as usize),
            TraceInterval::Last(from_end) => (
                self.ticks.len().saturating_sub(from_end as usize),
                self.ticks.len(),
            ),
            TraceInterval::Between(start, end) => (start as usize, end as usize),
        };

        let end = end.min(self.ticks.len());
        start..end
    }
}
