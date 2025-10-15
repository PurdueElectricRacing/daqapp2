mod app;
mod bootloader;
mod can_viewer;
mod log_parser;
mod scope;
mod shortcuts;
mod sidebar;
mod widgets;
mod workspace;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "DaqApp2",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(app::DAQApp::default()))),
    )
}
