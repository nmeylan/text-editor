#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![feature(trait_upcasting)]

#[macro_use]
extern crate elapsed_time;

pub mod text_editor;