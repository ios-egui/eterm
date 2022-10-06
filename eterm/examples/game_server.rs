//! Example for something spinning fast (~60 Hz) and server
//! a eterm at the same time:

use std::{thread, time::Duration};

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let mut eterm_server = eterm::Server::new("0.0.0.0:8505").unwrap();

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

        // With every eterm_server.show() the ui is updated internally
        // but in order to preserve bandwidht, the client is only updated
        // every miminum_update_interval, or earlier if there was input from
        // the client

        // Set the sleep time to a duration that is lower than the maximum_response_time
        // of the client, otherwise the response time will be lower than the clients wants
        thread::sleep(Duration::from_secs_f32(1.0 / 60.0));
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
