use core::time;

use tracing::info;

mod hooks;
mod script;
mod tas_player;
mod windows_input;

#[ctor::ctor]
fn main() {
    if let Some(arg) = std::env::args().next_back() {
        if arg == "witness64_d3d11.exe" {
            std::thread::spawn(|| setup());
        }
    }
}

fn setup() {
    std::thread::sleep(time::Duration::from_secs(1));

    let file_appender = tracing_appender::rolling::never(".", format!("witness_tas.log"));
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_target(false)
        .init();

    // If we don't do this, the logger dies at the end of this thread.
    // We want it to live for our hooks even when the init is done.
    std::mem::forget(_guard);

    info!("Starting initialiser thread.");

    hooks::init_hooks();
    hooks::enable_hooks();

    info!("Initialiser thread done.");
}
