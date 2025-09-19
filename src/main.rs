mod app;
mod sidebar;
mod workspace;
mod widgets;
mod can_viewer;
mod bootloader;
mod live_plot;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "DAQ App",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(app::DAQApp::default()))),
    )
}