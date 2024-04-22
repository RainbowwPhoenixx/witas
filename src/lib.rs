use tracing::info;

mod hooks;

#[ctor::ctor]
fn main() {
    let info = std::env::args().next_back();

    if let Some(arg) = info {
        if arg == "witness64_d3d11.exe" {
            std::thread::spawn(|| setup());
        }
    }
}

fn setup() {
    let file_appender = tracing_appender::rolling::never(".", format!("witness_tas.log"));
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .with_writer(non_blocking)
        .init();

    // If we don't do this, the logger dies at the end of this thread.
    // We want it to live for our hooks even when the init is done.
    std::mem::forget(_guard);

    info!("Starting initialiser thread.");

    hooks::init_hooks();
    hooks::enable_hooks();

    info!("Initialiser thread done.");
}
