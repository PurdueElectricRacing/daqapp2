mod app;
mod shortcuts;
mod ui;
mod widgets;
mod workspace;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "DaqApp2",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(app::DAQApp::default()))),
    )
}
