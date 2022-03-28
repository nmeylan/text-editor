use eframe::{egui, epi};

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Empty app", options, Box::new(|cc| {
        Box::new(MyApp::default())
    }));
}

#[derive(Default)]
struct MyApp{}

impl eframe::epi::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
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