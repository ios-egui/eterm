/// eterm viewer viewer.
///
/// Connects to an eterm server somewhere.
#[derive(argh::FromArgs)]
struct Arguments {
    /// which server to connect to, e.g. `127.0.0.1:8505`.
    #[argh(option)]
    url: String,
}

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let opt: Arguments = argh::from_env();
    eterm_viewer::run(opt.url)
}
