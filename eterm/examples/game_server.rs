//! Example for something spinning fast (~60 Hz) and server
//! a eterm at the same time:
use std::thread;

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let mut eterm_server = eterm::Server::new("0.0.0.0:8505").unwrap();

    // With every eterm_server.show() the ui is updated internally.
    // The eterm client receives a new frame every at lease every miminum_update_interval,
    // The default is 1 second.
    // When the eterm server receives input from an eterm client (e.g. mouse events)
    // the server will send updates with a maximum_frame_rate, the default is
    // 60 frames per second.

    // you can change the minimum update interval with:
    // eterm_server.set_minimum_update_interval(<Duration>);

    let mut demo_windows = egui_demo_lib::DemoWindows::default();

    loop {
        eterm_server
            .show(|egui_ctx: &egui::Context, _client_id: eterm::ClientId| {
                egui::TopBottomPanel::bottom("Standard Egui Demo").show(egui_ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Server time:");
                        ui_clock(ui);
                    });
                });
                demo_windows.ui(egui_ctx);
            })
            .unwrap();

        // Set the sleep time duration equal or lower than the max frame rate
        // for the client, otherwise the client will expience slower response times than
        // expected.
        thread::sleep(eterm::DEFAULT_MAX_UPDATE_INTERVAL);
    }
}

fn ui_clock(ui: &mut egui::Ui) {
    let seconds_since_midnight = seconds_since_midnight();

    ui.monospace(format!(
        "{:02}:{:02}:{:02}",
        (seconds_since_midnight % (24.0 * 60.0 * 60.0) / 3600.0).floor(),
        (seconds_since_midnight % (60.0 * 60.0) / 60.0).floor(),
        (seconds_since_midnight % 60.0).floor(),
    ));
}

fn seconds_since_midnight() -> f64 {
    use chrono::Timelike;
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}
