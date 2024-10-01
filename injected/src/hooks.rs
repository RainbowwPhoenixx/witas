use common::tas::PlaybackState;
use rand::Rng;
use retour::static_detour;
use std::ptr::addr_of;
use std::time;
use std::{ffi::CStr, marker::PhantomData};
use tracing::{debug, error, info};
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::{Foundation::POINT, UI::Input::RAWINPUT};

use crate::tas_player::TAS_PLAYER;
use crate::witness::windows_types::{Message, VirtualKeyCode};
use crate::witness::witness_types::{Color, Entity, Vec2, Vec3};

/// Inits a list of hooks.
///
/// Replaces the function at address with our function using the static detour.
/// Returns false if at least one of the hooks failed to initialize, true otherwise
///
/// # Examples
///
/// ```no_run
/// init_hook!(
///     static_detour1 @ address1 -> our_function1,
///     static_detour2 @ address2 -> our_function2,
/// );
/// ```
macro_rules! init_hook {
    ( $( $hook: ident @ $target_addr: literal -> $detour: expr ),* $(,)? ) => {
        {
            let mut succeed = true;
            $(
                let name = stringify!($hook);
                if let Err(e) =
                    unsafe { $hook.initialize(std::mem::transmute($target_addr as usize), $detour) }
                {
                    error!("Failed to init hook {name}! {e}");
                    succeed = false;
                }
            )*
            succeed
        }
    };
}

macro_rules! enable_hook {
    ( $( $hook: ident ),* $(,)? ) => {
        {
            let mut succeed = true;
            $(
                let name = stringify!($hook);
                if let Err(e) = unsafe { $hook.enable() } {
                    error!("Failed to enable hook {name}! {e}");
                    succeed = false
                } else {
                    debug!("Enabled hook {name}")
                }
            )*
            succeed
        }
    };
}

macro_rules! disable_hook {
    ( $( $hook: ident),* $(,)? ) => {
        {
            let mut succeed = true;
            $(
                let name = stringify!($hook);
                if let Err(e) = unsafe { $hook.disable() } {
                    error!("Failed to disable hook {name}! {e}");
                    succeed = false
                } else {
                    debug!("Disabled hook {name}")
                }
            )*
            succeed
        }
    };
}

pub struct PointerChain<'a, T>(&'a [usize], PhantomData<T>);

impl<'a, T: Copy> PointerChain<'a, T> {
    const fn new(chain: &'a [usize]) -> Self {
        Self(chain, PhantomData)
    }

    unsafe fn resolve_addr(&self) -> usize {
        let mut addr = self.0[0];
        for offset in &self.0[1..] {
            addr = *(addr as *const _);
            addr += offset;
        }
        addr
    }

    /// Resolve the pointer chain and return the value
    ///
    /// # Safety
    /// This function is unsafe because it manipulates raw pointers
    pub unsafe fn read(&self) -> T {
        let addr = self.resolve_addr();

        *(addr as *const T)
    }

    /// Resolve the pointer chain and write the new
    ///
    /// # Safety
    /// This function is unsafe because it manipulates raw pointers
    pub unsafe fn write(&self, value: T) {
        let addr = self.resolve_addr();

        *(addr as *mut T) = value;
    }
}

// Hooking shit, split because recursion limit
static_detour! {
    static MainLoopStart: unsafe extern "win64" fn();
    static GetInput: unsafe extern "win64" fn(usize, *const RAWINPUT) -> u64;
    static HandleAllMessages: unsafe extern "win64" fn(usize, u64);
    static HandleMessage: unsafe extern "win64" fn(usize, *const MSG) -> u64;
    static DeclareConsoleCommands: unsafe extern "win64" fn();
    static DeclareConsoleCommand: unsafe extern "win64" fn(usize, usize, *mut i8, u32, u32);
    static HandleKeyboardInput: unsafe extern "win64" fn(usize, u8, u8, u8, u32, u32) -> u64;

    // Used by the game to check key presses
    static IsKeyPressed: unsafe extern "win64" fn(usize, u32) -> u32;
    static GetMouseDeltaPos: unsafe extern "win64" fn(usize, *mut i32,  *mut i32,  *mut i32, bool);

    // For newgame tas start mode
    pub static DoRestart: unsafe extern "win64" fn();

    static WindowProcCallback: unsafe extern "win64" fn(usize, u32, u64, u64) -> usize;
}

// Damn you recursion limit
static_detour! {
    // Drawing stuff
    static drawScreen: unsafe extern "win64" fn();
    static drawSphere: unsafe extern "win64" fn(*const Vec3, f32, Color<f32>, bool);
    static drawCylinder: unsafe extern "win64" fn(*const Vec3, *const Vec3, f32, Color<f32>, u32);
    static MiddleOfDrawing: unsafe extern "win64" fn(usize, usize);

    static SetCursorToPos: unsafe extern "win64" fn(u64, u32, u32);

    static PanelClickCheck: unsafe extern "win64" fn(*const u32, *const Vec3, *const Vec3, *const f32, f32, bool, u8, u8) -> *const Entity;

    pub static CopyString: unsafe extern "win64" fn(*const i8) -> *mut i8;

    // rng stuff
    static GetRandomFloatInRange: unsafe extern "win64" fn(*mut u32, f32, f32) -> f32;
}

static PTR_INPUT_STH1: PointerChain<usize> = PointerChain::new(&[0x14469a060, 0x0]);
static mut HANDLE_MSG_PARAM1: Option<usize> = None;
// static MOUSE_COORDS_FROM_SCREEN_CENTER: PointerChain<(i32, i32)> = PointerChain::new(&[0x14469a060, 0x0, 0x9c]);
pub static MAIN_LOOP_COUNT: PointerChain<u32> = PointerChain::new(&[0x14062d5c8]);
pub static NEW_GAME_FLAG: PointerChain<bool> = PointerChain::new(&[0x14062d076]);
pub static DEBUG_SHOW_EPS: PointerChain<bool> = PointerChain::new(&[0x140630410]);
pub static FRAMETIME: PointerChain<f64> = PointerChain::new(&[0x1406211d8]);
pub static PLAYER: PointerChain<Entity> = PointerChain::new(&[0x14062d0a0, 0x18, 0x1E465 * 8, 0x0]);
pub static NOCLIP: PointerChain<bool> = PointerChain::new(&[0x14062d5b8]);
pub static PLAYER_POS: PointerChain<Vec3> =
    PointerChain::new(&[0x14062d0a0, 0x18, 0x1E465 * 8, 0x24]);
pub static PLAYER_ANG: PointerChain<Vec2> = PointerChain::new(&[0x1406303ec]);
pub static INTERACTION_STATUS: PointerChain<u32> = PointerChain::new(&[0x14062d5c4]);
pub static VERTICAL_SMOOTHING: PointerChain<f32> = PointerChain::new(&[0x140630434]);

static mut SAVED_CAM_POS: Option<Vec3> = None;
static mut SAVED_CAM_ANG: Option<Vec2> = None;

// Holds the real tabbed out value, since we make the game forget
static mut TABBED_OUT: bool = false;

// Variables for save loading
pub static mut SAVE_PATH: PointerChain<*mut i8> = PointerChain::new(&[0x14062d790]);
pub static LOAD_SAVE_FLAG: PointerChain<bool> = PointerChain::new(&[0x14062d789]);
pub static mut APPDATA_PATH: PointerChain<*mut i8> = PointerChain::new(&[0x14062d778]);

pub static mut RNG_SEED: PointerChain<*mut u32> = PointerChain::new(&[0x14062d0b0]);

// ------------------------------------------------------------------------------------
//                                 OUR OVERRIDES
// ------------------------------------------------------------------------------------

// Anything in here will be executed at the beginning
// of every iteration of the main loop
fn execute_tas_inputs() {
    let mut player = TAS_PLAYER.lock().unwrap();
    let input_sth1 = unsafe { PTR_INPUT_STH1.read() };

    match unsafe { (player.as_mut(), HANDLE_MSG_PARAM1) } {
        (Some(tas_player), Some(handle_message_this)) => unsafe {
            if let Some(controller) = tas_player.get_controller() {
                // Movement
                match (controller.current.forward, controller.previous.forward) {
                    (true, false) => {
                        HandleKeyboardInput.call(input_sth1, 0, 0, 1, VirtualKeyCode::W as u32, 0)
                    }
                    (false, true) => {
                        HandleKeyboardInput.call(input_sth1, 0, 0, 0, VirtualKeyCode::W as u32, 0)
                    }
                    _ => 0,
                };
                match (controller.current.backward, controller.previous.backward) {
                    (true, false) => {
                        HandleKeyboardInput.call(input_sth1, 0, 0, 1, VirtualKeyCode::S as u32, 0)
                    }
                    (false, true) => {
                        HandleKeyboardInput.call(input_sth1, 0, 0, 0, VirtualKeyCode::S as u32, 0)
                    }
                    _ => 0,
                };
                match (controller.current.left, controller.previous.left) {
                    (true, false) => {
                        HandleKeyboardInput.call(input_sth1, 0, 0, 1, VirtualKeyCode::A as u32, 0)
                    }
                    (false, true) => {
                        HandleKeyboardInput.call(input_sth1, 0, 0, 0, VirtualKeyCode::A as u32, 0)
                    }
                    _ => 0,
                };
                match (controller.current.right, controller.previous.right) {
                    (true, false) => {
                        HandleKeyboardInput.call(input_sth1, 0, 0, 1, VirtualKeyCode::D as u32, 0)
                    }
                    (false, true) => {
                        HandleKeyboardInput.call(input_sth1, 0, 0, 0, VirtualKeyCode::D as u32, 0)
                    }
                    _ => 0,
                };

                // Running
                match (controller.current.running, controller.previous.running) {
                    (true, false) => HandleKeyboardInput.call(
                        input_sth1,
                        0,
                        0,
                        1,
                        VirtualKeyCode::LShift as u32,
                        0,
                    ),
                    (false, true) => HandleKeyboardInput.call(
                        input_sth1,
                        0,
                        0,
                        0,
                        VirtualKeyCode::LShift as u32,
                        0,
                    ),
                    _ => 0,
                };

                // Puzzle mode toggle
                let msg_template = MSG {
                    hwnd: windows::Win32::Foundation::HWND(0),
                    message: Message::WM_LBUTTONDOWN as u32,
                    wParam: windows::Win32::Foundation::WPARAM(1),
                    lParam: windows::Win32::Foundation::LPARAM(23593600),
                    time: 0,
                    pt: POINT { x: 1000, y: 1000 },
                };
                match (
                    controller.current.left_click,
                    controller.previous.left_click,
                ) {
                    (true, false) => {
                        let msg = MSG {
                            message: Message::WM_LBUTTONDOWN as u32,
                            ..msg_template
                        };
                        HandleMessage.call(handle_message_this, addr_of!(msg))
                    }
                    (false, true) => {
                        let msg = MSG {
                            message: Message::WM_LBUTTONUP as u32,
                            ..msg_template
                        };
                        HandleMessage.call(handle_message_this, addr_of!(msg))
                    }
                    _ => 0,
                };
                match (
                    controller.current.right_click,
                    controller.previous.right_click,
                ) {
                    (true, false) => {
                        let msg = MSG {
                            message: Message::WM_RBUTTONDOWN as u32,
                            ..msg_template
                        };
                        HandleMessage.call(handle_message_this, addr_of!(msg))
                    }
                    (false, true) => {
                        let msg = MSG {
                            message: Message::WM_RBUTTONUP as u32,
                            ..msg_template
                        };
                        HandleMessage.call(handle_message_this, addr_of!(msg))
                    }
                    _ => 0,
                };
            }
        },
        _ => error!("Failed to execute TAS inputs."),
    }
}

fn get_input(this: usize, hrawinput: *const RAWINPUT) -> u64 {
    // let val = unsafe { *hrawinput };
    // info!("get_input: {val:#?}");
    unsafe { GetInput.call(this, hrawinput) }
}

fn handle_all_messages(this: usize, idk: u64) {
    unsafe { HANDLE_MSG_PARAM1 = Some(this) };
    unsafe { HandleAllMessages.call(this, idk) }

    // Using a mix of keyboard inputs and messages isn't very good here, TODO: make it use the same mechanism
    execute_tas_inputs();
}

fn handle_message(this: usize, message: *const MSG) -> u64 {
    // During tas playback, ignore all user messages
    if let Ok(player) = TAS_PLAYER.lock() {
        if let Some(player) = player.as_ref() {
            if player.get_playback_state() != PlaybackState::Stopped {
                return 0; // The game always returns 0 in this function, let's do the same
            }
        }
    };

    let val = unsafe { *message }.message;

    match Message::try_from(val) {
        Ok(msg) => match msg {
            Message::WM_INPUT => {}
            Message::WM_KEYDOWN => return 0,
            Message::WM_KEYUP => return 0,
            Message::WM_CHAR => return 0,
            Message::WM_DEADCHAR => return 0,
            Message::WM_SYSKEYDOWN => {}
            Message::WM_SYSKEYUP => return 0,
            Message::WM_MOUSEMOVE => {}
            Message::WM_LBUTTONDOWN => {}
            Message::WM_LBUTTONUP => {}
            Message::WM_LBUTTONDLBCLK => {}
            Message::WM_RBUTTONDOWN => {}
            Message::WM_RBUTTONUP => {}
            Message::WM_RBUTTONDBCLK => {}
            _ => debug!("This message is recieved! {msg:#?}"),
        },
        Err(_) => debug!("handle_message: unkown message {val}"),
    }

    unsafe { HandleMessage.call(this, message) }
}

fn declare_console_commands() {
    unsafe { DeclareConsoleCommands.call() }
}

fn declare_console_command(this: usize, func: usize, name: *mut i8, arg_type: u32, arg_count: u32) {
    let name_str = String::from_utf8_lossy(unsafe { CStr::from_ptr(name as *const i8) }.to_bytes());
    info!("declare_console_command: {name_str}");
    unsafe { DeclareConsoleCommand.call(this, func, name, arg_type, arg_count) }
}

fn handle_keyboard_input(
    this: usize,
    ri_key_break: u8,
    ri_key_e0: u8,
    press_down: u8,
    virtual_keycode: u32,
    scan_code: u32,
) -> u64 {
    if virtual_keycode == VirtualKeyCode::P as u32 && press_down == 1 {
        if let Some(tas_player) = TAS_PLAYER.lock().unwrap().as_mut() {
            tas_player.start(None)
        }
    }

    if virtual_keycode == VirtualKeyCode::N as u32 && press_down == 1 {
        unsafe { NOCLIP.write(!NOCLIP.read()) };
    }

    if virtual_keycode == VirtualKeyCode::J as u32 && press_down == 1 {
        unsafe {
            SAVED_CAM_POS = Some(PLAYER_POS.read());
            SAVED_CAM_ANG = Some(PLAYER_ANG.read());
        };
        info!("Saved camera pos")
    }
    if virtual_keycode == VirtualKeyCode::K as u32 && press_down == 1 {
        unsafe {
            if let (Some(pos), Some(ang)) = (SAVED_CAM_POS, SAVED_CAM_ANG) {
                PLAYER_POS.write(pos);
                PLAYER_ANG.write(ang);
            }
        };
    }

    if virtual_keycode == VirtualKeyCode::E as u32 && press_down == 1 {
        unsafe { DEBUG_SHOW_EPS.write(!DEBUG_SHOW_EPS.read()) }
    }

    unsafe {
        HandleKeyboardInput.call(
            this,
            ri_key_break,
            ri_key_e0,
            press_down,
            virtual_keycode,
            scan_code,
        )
    }
}

fn is_key_pressed(this: usize, key: u32) -> u32 {
    let res = unsafe { IsKeyPressed.call(this, key) };

    if res != 0 {
        info!("check_key {res}: {key:#X}");
    }

    res
}

fn get_mouse_delta_pos(
    input_thing: usize,
    mouse_x_out: *mut i32,
    mouse_y_out: *mut i32,
    mwheel_out: *mut i32,
    idk: bool,
) {
    unsafe { GetMouseDeltaPos.call(input_thing, mouse_x_out, mouse_y_out, mwheel_out, idk) };

    // Override the values during tas replay
    let mut player = match TAS_PLAYER.lock() {
        Ok(player) => player,
        Err(_) => return,
    };

    let controller = match player.as_mut() {
        Some(tas_player) => tas_player.get_controller(),
        None => return,
    };

    let mouse_coords = match controller {
        Some(controller) => controller.current.mouse_pos,
        None => return,
    };

    unsafe {
        *mouse_x_out = mouse_coords.0;
        *mouse_y_out = mouse_coords.1;
    }
}

fn draw_override() {
    // Don't draw during skipping
    if let Ok(player) = TAS_PLAYER.lock() {
        if let Some(player) = player.as_ref() {
            if player.should_do_skipping() {
                return;
            }
        }
    };

    let before_draw = time::Instant::now();

    unsafe {
        drawScreen.call();
    }

    // Do the frame by frame
    if let Ok(mut player) = TAS_PLAYER.lock() {
        if let Some(player) = player.as_mut() {
            if player.get_playback_state() == PlaybackState::Paused {
                player.block_until_next_frame();
            }
        }
    };

    // Do the vsync ourselves, makes lag less bad
    let remaining = 16u64.saturating_sub(before_draw.elapsed().as_millis() as u64);
    std::thread::sleep(time::Duration::from_millis(remaining))
}

fn middle_of_drawing(param1: usize, param2: usize) {
    // Call original function. The appropriate calling parameters should
    // already be set by whoever is calling us. We don't need to care about
    // them, we just want to briefly jump to our own code after
    unsafe { MiddleOfDrawing.call(param1, param2) }

    unsafe {
        if let Ok(player) = TAS_PLAYER.lock() {
            if let Some(player) = player.as_ref() {
                // Draw positions
                for tick_data in player.trace.get_pos_to_show() {
                    let color = match tick_data.interact {
                        crate::witness::witness_types::InteractionStatus::FocusMode => Color::RED,
                        crate::witness::witness_types::InteractionStatus::SolvingPanel => {
                            Color::PINK
                        }
                        crate::witness::witness_types::InteractionStatus::Walking => Color::GREEN,
                        crate::witness::witness_types::InteractionStatus::Cinematic => Color {
                            r: 0.5,
                            g: 0.5,
                            b: 0.5,
                            a: 0.5,
                        },
                    };

                    let mut pos = tick_data.pos;
                    pos.z += player.trace.draw_option.z_offset;

                    drawSphere.call(
                        addr_of!(pos),
                        player.trace.draw_option.sphere_radius,
                        color,
                        false,
                    );
                }

                // Draw puzzls clicks
                let color = Color::BLUE;
                for (pos, dir) in player.trace.get_puzzle_clicks() {
                    let pos = pos
                        + dir
                            * player
                                .trace
                                .draw_option
                                .puzzle_click_indicator_distance_multiplier;
                    drawSphere.call(
                        addr_of!(pos),
                        player.trace.draw_option.puzzle_click_indicator_radius,
                        color,
                        false,
                    );
                }
            }
        };
    }
}

fn window_proc_callback(idc: usize, msg: u32, wparam: u64, lparam: u64) -> usize {
    // Override the message indicating lost focus, make the game think it is focused always
    match msg {
        0x86 => unsafe {
            TABBED_OUT = wparam == 0;
            WindowProcCallback.call(idc, msg, 1, lparam)
        },
        _ => unsafe { WindowProcCallback.call(idc, msg, wparam, lparam) },
    }
}

fn set_cursor_to_pos(idk: u64, x: u32, y: u32) {
    // When tabbed out, dont grab cursor
    if unsafe { TABBED_OUT } {
        return;
    }

    unsafe { SetCursorToPos.call(idk, x, y) }
}

fn panel_click_check(
    this: *const u32,
    position: *const Vec3,
    direction: *const Vec3,
    idk4: *const f32,
    idk5: f32,
    not_induced_by_click: bool,
    idk7: u8,
    idk8: u8,
) -> *const Entity {
    static mut UNLOCKED_COUNT: u32 = 0;

    let new_unlocked_panels_count = unsafe { *this };

    if unsafe { UNLOCKED_COUNT } != new_unlocked_panels_count {
        unsafe { UNLOCKED_COUNT = new_unlocked_panels_count };
        if let Ok(mut tas_player) = TAS_PLAYER.lock() {
            if let Some(player) = tas_player.as_mut() {
                if player.get_playback_state() != PlaybackState::Stopped {
                    player.send_puzzle_unlock();
                }
            }
        }
    }

    let ret = unsafe {
        PanelClickCheck.call(
            this,
            position,
            direction,
            idk4,
            idk5,
            not_induced_by_click,
            idk7,
            idk8,
        )
    };

    if !not_induced_by_click {
        unsafe {
            if let Ok(mut tas_player) = TAS_PLAYER.lock() {
                if let Some(player) = tas_player.as_mut() {
                    if player.get_playback_state() != PlaybackState::Stopped {
                        player.add_puzzle_click(*position, *direction);
                    }
                }
            }
        }
    }

    ret
}

// This hook is a little cheaty but it fixes door rng so whatever
fn get_random_float_within_range(_seed: *mut u32, min: f32, max: f32) -> f32 {
    rand::thread_rng().gen::<f32>() * (max - min) + min
}

// ------------------------------------------------------------------------------------
//                                 ACTUAL HOOKING
// ------------------------------------------------------------------------------------

pub fn init_hooks() {
    // Make sure the memory is initialized (when this first runs, wine may not yet have put the game exe in memory)
    debug!("Waiting for wine to put the executable in memory.");
    let addr = 0x1401e5120 as *const u8;
    let raw_mem: &[u8] = unsafe { std::slice::from_raw_parts(addr, 10) };

    // This is not an infinite loop because the game itself writes stuff there
    #[allow(clippy::while_immutable_condition)]
    while raw_mem[0] == 0 {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    debug!("Done. Starting to initialise hooks.");

    // Placeholder function for functions we want to call without hooking
    fn placeholder_func() {
        error!("This function is not supposed to be hooked.");
        unreachable!()
    }

    let placeholder = placeholder_func as *const ();

    // Init the hooks
    let success = init_hook!(
        // MainLoopStart          @ 0x1401e5120 -> main_loop_begin,
        GetInput               @ 0x140346110 -> get_input,
        HandleAllMessages      @ 0x140345a60 -> handle_all_messages,
        HandleMessage          @ 0x140345bc0 -> handle_message,
        DeclareConsoleCommands @ 0x140071b20 -> declare_console_commands,
        DeclareConsoleCommand  @ 0x1402f58b0 -> declare_console_command,
        HandleKeyboardInput    @ 0x140344a60 -> handle_keyboard_input,
        IsKeyPressed           @ 0x1403448e0 -> is_key_pressed,
        GetMouseDeltaPos       @ 0x1403448f0 -> get_mouse_delta_pos,
        drawScreen             @ 0x1401c8970 -> draw_override,
        MiddleOfDrawing        @ 0x140274b10 -> middle_of_drawing,
        WindowProcCallback     @ 0x14037aea0 -> window_proc_callback,
        SetCursorToPos         @ 0x140346580 -> set_cursor_to_pos,
        PanelClickCheck        @ 0x140231550 -> panel_click_check,
        GetRandomFloatInRange  @ 0x1402f9a70 -> get_random_float_within_range,
        DoRestart              @ 0x1401f9e60 -> std::mem::transmute::<_, fn()>(placeholder),
        drawSphere             @ 0x1400761d0 -> std::mem::transmute::<_, fn(_,_,_,_)>(placeholder),
        drawCylinder           @ 0x1401c9b40 -> std::mem::transmute::<_, fn(_,_,_,_,_)>(placeholder),
        CopyString             @ 0x1402e9490 -> std::mem::transmute::<_, fn(_) -> _>(placeholder),
    );

    // Patch frametime to make physics consistent
    let nops = &[0x90_u8; 5];
    let frametime_set_addr = 0x1402e96cf as *const u8;
    unsafe {
        let _ = region::protect(
            frametime_set_addr,
            5,
            region::Protection::READ_WRITE_EXECUTE,
        );
        std::ptr::copy_nonoverlapping(nops.as_ptr(), frametime_set_addr as *mut u8, 5);
        FRAMETIME.write(0.0166666666);
    }

    if !success {
        error!("Failed to initialize hooks, abandonning loading.");
        panic!("Failed to load, see log for more details.")
    }
}

pub fn enable_hooks() {
    let success = enable_hook!(
        // MainLoopStart,
        GetInput,
        HandleAllMessages,
        HandleMessage,
        DeclareConsoleCommands,
        // DeclareConsoleCommand,
        HandleKeyboardInput,
        // IsKeyPressed,
        GetMouseDeltaPos,
        drawScreen,
        MiddleOfDrawing,
        WindowProcCallback,
        SetCursorToPos,
        PanelClickCheck,
        GetRandomFloatInRange,
    );

    if !success {
        error!("Failed to enable hooks, we may be partially loaded, expect weird behavior.");
    }
}

#[allow(unused)]
pub fn disable_hooks() {
    let success = disable_hook!(
        // MainLoopStart,
        GetInput,
        HandleAllMessages,
        HandleMessage,
        DeclareConsoleCommands,
        DeclareConsoleCommand,
        HandleKeyboardInput,
        IsKeyPressed,
        GetMouseDeltaPos,
        drawScreen,
        MiddleOfDrawing,
        WindowProcCallback,
        SetCursorToPos,
        PanelClickCheck,
        GetRandomFloatInRange,
    );

    if !success {
        error!("Failed to disable hooks. We'll just leave it be and hope for the best.");
    }
}
