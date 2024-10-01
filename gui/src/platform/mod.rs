/// This module contains platform-specific code

#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod os;

pub use os::*;

// unused: this is used only on windows (for now)
#[cfg(windows)]
fn get_library_path() -> std::path::PathBuf {
    use std::fs;

    // Contents of the dll to inject
    let lib_inject = include_bytes!(env!("CARGO_CDYLIB_FILE_INJECTED"));

    // Write file to temp dir
    let mut file_path = std::env::temp_dir();
    file_path.push("witness_tas.dll");
    let _ = fs::write(file_path.clone(), lib_inject);

    file_path
}
