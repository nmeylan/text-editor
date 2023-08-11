#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![feature(trait_upcasting)]


#[macro_use]
extern crate elapsed_time;

use std::env;
use eframe::{egui, epi};
use text_editor::text_editor::TextEditor;
use crate::egui::Rounding;

#[derive(Default, Debug, Clone)]
struct Pos<T> {
    x: T,
    y: T,
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Text editor", options, Box::new(|cc| {
        Box::new(MyApp::new(cc))
    }));
}

impl eframe::epi::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _: &epi::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.label("top panel");
        });
        egui::SidePanel::left("left").show(ctx, |ui| {
            ui.label("left panel");
        });
        let mut panel = egui::CentralPanel::default();
        let mut panel_frame = egui::containers::Frame {
            margin: Default::default(),
            rounding: Rounding::none(),
            fill: ctx.style().visuals.window_fill(),
            stroke: Default::default(),
            ..Default::default()
        };
        panel = panel.frame(panel_frame);
        panel.show(ctx, |ui| {
            self.text_editor.ui(ctx, ui);
        });
    }
}

struct MyApp {
    text_editor: TextEditor,
}
impl MyApp {
    fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let args: Vec<_> = env::args().collect();
        if args.len() < 2 {
            println!("Please provide file to open as 1st program argument");
        } else {
            println!("Opening {}", args[1].as_str());
        }

        Self {
            text_editor: TextEditor::new(creation_context, args[1].as_str()),
        }
    }
}
