mod app;
mod sidebar;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "DAQ App",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(app::DAQApp::default()))),
    )
}