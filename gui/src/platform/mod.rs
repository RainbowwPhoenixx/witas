/// This module contains platform-specific code

#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod os;

use std::path::PathBuf;

pub use os::*;

// unused: this is used only on windows
#[allow(unused)]
fn get_library_path() -> PathBuf {
    // TODO: make this include the dll bytes and write the dll/so to a file
    // before injection
    let base_path = PathBuf::from(env!("OUT_DIR"));

    #[cfg(windows)]
    let lib_name = "witness_tas.dll";
    #[cfg(unix)]
    let lib_name = "witness_tas.so";

    base_path.join(lib_name)
}
