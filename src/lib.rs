#![feature(panic_update_hook)]

use core::time;

use tracing::{error, info};

pub mod communication;
pub mod hooks;
pub mod script;
pub mod tas_player;
pub mod witness;

#[ctor::ctor]
fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        if arg == "witness64_d3d11.exe" {
            std::thread::spawn(setup);
        }
    }
}

fn setup() {
    std::thread::sleep(time::Duration::from_secs(1));

    let file_appender = tracing_appender::rolling::never(".", "witness_tas.log");
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
    tas_player::TasPlayer::init();

    std::panic::update_hook(move |prev, info| {
        if let Some(location) = info.location() {
            error!(
                "TAS tool panicked in file {} at line {}: {info}",
                location.file(),
                location.line()
            );
        } else {
            error!("TAS tool panicked: {info}")
        }

        prev(info)
    });

    info!("Initialiser thread done.");
}
