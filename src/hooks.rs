#![allow(unsupported_calling_conventions)]
use retour::static_detour;
use tracing::{debug, error, info};
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::Input::RAWINPUT;
use std::ffi::CStr;

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

static_detour! {
    static MainLoopStart: unsafe extern "win64" fn();
    static GetInput: unsafe extern "win64" fn(usize, *const RAWINPUT) -> u64;
    static HandleMessage: unsafe extern "win64" fn (usize, *const MSG) -> u64;
    static DeclareConsoleCommands: unsafe extern "win64" fn();
    static DeclareConsoleCommand: unsafe extern "win64" fn(usize, usize, usize, u32, u32);
    static HandleKeyboardInput: unsafe extern "win64" fn(usize, u8, u8, u8, u32, u32) -> u64;
}

// Anything in here will be executed at the beginning
// of every iteration of the main loop
fn main_loop_begin() {}

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

fn declare_console_command(_idk: usize, func: usize, name: usize, arg_type: u32, arg_count: u32) {
    let name_str = String::from_utf8_lossy(unsafe { CStr::from_ptr(name as *const i8) }.to_bytes());
    info!("declare_console_command: {name_str}");
    unsafe { DeclareConsoleCommand.call(_idk, func, name, arg_type, arg_count) }
}

// fn handle_keyboard_input(_idk1: usize, _idk2: u8, _idk3: u8, press_down: u8, _idk5: u32, _idk6: u32) -> u64 {
//     info!(_idk2, _idk3, press_down, _idk5, _idk6);
//     unsafe { HandleKeyboardInput.call(_idk1, _idk2, _idk3, press_down, _idk5, _idk6) }
// }

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
        MainLoopStart @ 0x1401e5120 -> main_loop_begin,
        GetInput      @ 0x140346110 -> get_input,
        HandleMessage @ 0x140345bc0 -> handle_message,
        DeclareConsoleCommands @ 0x140071b20 -> declare_console_commands,
        DeclareConsoleCommand  @ 0x1402f58b0 -> declare_console_command,
        // ,HandleKeyboardInput    @ 0x140344a60 -> handle_keyboard_input
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
        HandleMessage,
        DeclareConsoleCommands,
        DeclareConsoleCommand,
        // HandleKeyboardInput
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
        // HandleKeyboardInput
    );

    if !success {
        error!("Failed to disable hooks. We'll just leave it be and hope for the best.");
    }
}
