use retour::static_detour;
use std::ffi::CStr;
use tracing::{debug, error, info};
use windows::Win32::UI::Input::RAWINPUT;
use windows::Win32::UI::WindowsAndMessaging::MSG;

use crate::tas_player::{TasPlayer, TAS_PLAYER};

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
    ( $( $hook: ident @ $target_addr: literal -> $detour: ident ),* $(,)? ) => {
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

// Some useful defs
#[allow(unused)]
#[repr(C)]
enum KeyCode {
    // Movement keys
    Z = 0x5A,
    Q = 0x51,
    S = 0x32,
    D = 0x44,

    // Play tas
    P = 0x50,
}

// Hooking shit
static_detour! {
    static MainLoopStart: unsafe extern "win64" fn();
    static GetInput: unsafe extern "win64" fn(usize, *const RAWINPUT) -> u64;
    static HandleMessage: unsafe extern "win64" fn (usize, *const MSG) -> u64;
    static DeclareConsoleCommands: unsafe extern "win64" fn();
    static DeclareConsoleCommand: unsafe extern "win64" fn(usize, usize, usize, u32, u32);
    static HandleKeyboardInput: unsafe extern "win64" fn(usize, u8, u8, u8, u32, u32) -> u64;
    static IsKeyPressed: unsafe extern "win64" fn(usize, u32) -> u32;
}

static mut INPUT_STUFF_PTR: Option<usize> = None;

// ------------------------------------------------------------------------------------
//                                 OUR OVERRIDES
// ------------------------------------------------------------------------------------

// Anything in here will be executed at the beginning
// of every iteration of the main loop
fn main_loop_begin() {
    let mut player = unsafe { TAS_PLAYER.lock().unwrap() };
    match unsafe { (INPUT_STUFF_PTR, player.as_mut()) } {
        (Some(addr), Some(tas_player)) => unsafe {
            if let Some(controller) = tas_player.get_controller() {
                HandleKeyboardInput.call(
                    addr,
                    0,
                    0,
                    controller.forward as u8,
                    KeyCode::Z as u32,
                    0,
                );
                HandleKeyboardInput.call(
                    addr,
                    0,
                    0,
                    controller.backward as u8,
                    KeyCode::S as u32,
                    0,
                );
                HandleKeyboardInput.call(addr, 0, 0, controller.left as u8, KeyCode::Q as u32, 0);
                HandleKeyboardInput.call(addr, 0, 0, controller.right as u8, KeyCode::D as u32, 0);
            }
        },
        _ => {}
    }
}

fn get_input(this: usize, hrawinput: *const RAWINPUT) -> u64 {
    // let val = unsafe { *hrawinput };
    // info!("get_input: {val:#?}");
    unsafe { GetInput.call(this, hrawinput) }
}

fn handle_message(this: usize, message: *const MSG) -> u64 {
    // let val = unsafe { *message };
    // info!("handle_message: {val:#?}");
    unsafe { HandleMessage.call(this, message) }
}

fn declare_console_commands() {
    unsafe { DeclareConsoleCommands.call() }
}

fn declare_console_command(this: usize, func: usize, name: usize, arg_type: u32, arg_count: u32) {
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
    // info!(
    //     ri_key_break,
    //     ri_key_e0, press_down, virtual_keycode, scan_code
    // );
    unsafe { INPUT_STUFF_PTR = Some(this) };

    if virtual_keycode == KeyCode::P as u32 && press_down == 1 {
        unsafe {
            let mut tas_player = TAS_PLAYER.lock().unwrap();

            *tas_player = TasPlayer::from_file(
                "/home/rainbow/Documents/dev/the witness/witness-tas/example.wtas".to_owned(),
            );

            tas_player.as_mut().unwrap().start(0)
        }
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

// ------------------------------------------------------------------------------------
//                                 ACTUAL HOOKING
// ------------------------------------------------------------------------------------

pub fn init_hooks() {
    // Make sure the memory is initialized (when this runs, wine has not yet put the game exe in memory)
    debug!("Waiting for wine to put the executable in memory.");
    let addr = 0x1401e5120 as *const u8;
    let raw_mem: &[u8] = unsafe { std::slice::from_raw_parts(addr, 10) };
    while raw_mem[0] == 0 {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    debug!("Done. Starting to initialise hooks.");

    // Init the hooks
    let success = init_hook!(
        MainLoopStart          @ 0x1401e5120 -> main_loop_begin,
        GetInput               @ 0x140346110 -> get_input,
        HandleMessage          @ 0x140345bc0 -> handle_message,
        DeclareConsoleCommands @ 0x140071b20 -> declare_console_commands,
        DeclareConsoleCommand  @ 0x1402f58b0 -> declare_console_command,
        HandleKeyboardInput    @ 0x140344a60 -> handle_keyboard_input,
        IsKeyPressed           @ 0x1403448e0 -> is_key_pressed,
    );

    if !success {
        error!("Failed to initialize hooks, abandonning loading.");
        panic!("Failed to load, see log for more details.")
    }
}

pub fn enable_hooks() {
    let success = enable_hook!(
        MainLoopStart,
        GetInput,
        // HandleMessage,
        DeclareConsoleCommands,
        // DeclareConsoleCommand,
        HandleKeyboardInput,
        // IsKeyPressed,
    );

    if !success {
        error!("Failed to enable hooks, we may be partially loaded, expect weird behavior.");
    }
}

#[allow(unused)]
pub fn disable_hooks() {
    let success = disable_hook!(
        MainLoopStart,
        GetInput,
        HandleMessage,
        DeclareConsoleCommands,
        DeclareConsoleCommand,
        HandleKeyboardInput,
        IsKeyPressed,
    );

    if !success {
        error!("Failed to disable hooks. We'll just leave it be and hope for the best.");
    }
}
