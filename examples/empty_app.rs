use eframe::{egui};

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Empty app", options, Box::new(|cc| {
        Box::new(MyApp::default())
    }));
}

#[derive(Default)]
struct MyApp{}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.label("top panel");
        });
        egui::SidePanel::left("left").show(ctx, |ui| {
            ui.label("left panel");
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello")
        });
    }
}