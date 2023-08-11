use std::any::Any;
use std::borrow::Borrow;
use std::cell::{RefCell, RefMut};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::fmt::format;
use std::default::Default;
use std::fs;
use std::ops::ControlFlow;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use glow_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, GlyphCruncher, Section, Text};
use eframe::emath::Vec2;
use eframe::egui::epaint::TextShape;
use eframe::egui::{Color32, Context, FontId, Galley, Pos2, Sense, TextFormat};
use eframe::egui::text::LayoutJob;
use eframe::egui::{*};
use eframe::epaint::{*};
use eframe::{egui, epaint, emath, CreationContext};
use glow_glyph::ab_glyph::{PxScale, Font, ScaleFont};
use crate::text_editor::SingleAction::NewLine;

pub struct TextEditor {
    lines: Vec<String>,
    glyph_brush_text_editor: Arc<Mutex<GlyphBrush>>,
    glyph_brush_line_number: Arc<Mutex<GlyphBrush>>,
    scroll_offset: Pos<f32>,
    lines_count: usize,
    char_width: f32,
    line_height: f32,
    scale: f32,
    gutter_width: f32,
    has_pressed_arrow_key: bool,
    text_editor_viewport: Rect,
    // Cursor
    cursor_index: Pos<usize>,
    cursor_pos: Pos<f32>,
    // Selection
    start_dragged_index: Option<Pos<usize>>,
    stop_dragged_index: Option<Pos<usize>>,
    selection_start_index: Option<Pos<usize>>,
    selection_end_index: Option<Pos<usize>>,
    highlighted_word: Option<String>,
    word_occurrences: RefCell<Vec<(Pos<usize>, Pos<usize>)>>,
    // matching open-close characters
    opening_char: RefCell<Option<char>>,
    closing_char: RefCell<Option<char>>,
    opening_char_index: RefCell<Option<Pos<usize>>>,
    closing_char_index: RefCell<Option<Pos<usize>>>,
    unsaved_stated: Option<UnsavedState>,
    history: Vec<State>,
    history_index: usize,
    latest_change_time: f32,
}

#[derive(Clone, Debug)]
enum SingleAction {
    AddChar(AddCharAction),
    RemoveChar(RemoveCharAction),
    RemoveLine(usize),
    NewLine(Pos<usize>),
}

#[derive(Clone, Debug)]
enum BulkAction {
    AddText(TextAction),
    RemoveText(TextAction),
}

#[derive(Default, Clone, Debug)]
pub struct TextAction {
    start_index: usize,
    end_index: usize,
    lines: Vec<String>,
}

#[derive(Default, Clone, Debug)]
pub struct AddCharAction {
    start_pos: Pos<usize>,
    char: String,
}

#[derive(Default, Clone, Debug)]
pub struct RemoveCharAction {
    start_pos: Pos<usize>,
    char: char,
}

#[derive(Default, Clone, Debug)]
pub struct DefaultAction {
    start_pos: Pos<usize>,
    line: String,
}

#[derive(Default, Clone, Debug)]
pub struct UnsavedState {
    last_activity_at: f64,
    cursor_index: Pos<usize>,
    cursor_pos: Pos<f32>,
    actions: Vec<SingleAction>,
}

#[derive(Clone, Debug)]
pub struct State {
    created_at: f64,
    cursor_index: Pos<usize>,
    cursor_pos: Pos<f32>,
    bulk_action: BulkAction,
}

#[derive(Default, Debug, Clone)]
pub struct Pos<T> {
    pub x: T,
    pub y: T,
}
const scale_factor: f32 = 1.5;
impl TextEditor {
    pub fn new(creation_context: &eframe::CreationContext<'_>, file_path: &str) -> Self {
        let font = ab_glyph::FontArc::try_from_slice(include_bytes!(
            "Inconsolata-Regular.ttf"
        )).unwrap();

        let glyph_brush = Arc::new(Mutex::new(GlyphBrushBuilder::using_font(font.clone())
            .initial_cache_size((2048 * 2, 2048 * 2))
            .draw_cache_position_tolerance(1.0)
            .build(creation_context.gl.as_ref().unwrap())));
        let glyph_brush_line_number = Arc::new(Mutex::new(GlyphBrushBuilder::using_font(font.clone())
            .initial_cache_size((120, 120))
            .draw_cache_position_tolerance(1.0)
            .build(creation_context.gl.as_ref().unwrap())));

        // let content = fs::read_to_string(Path::new("/Users/nmeylan/dev/perso/meta-editor/nmeylan/src/text")).unwrap();
        let content = fs::read_to_string(Path::new(file_path)).unwrap();
        // let content = fs::read_to_string(Path::new("/home/nmeylan/dev/perso/rust-ragnarok-server/lib/packets/src/packets_impl.rs")).unwrap();
        let split = content.split("\n").map(|s| s.to_string()).collect::<Vec<String>>();
        let lines_count = split.len();
        let font_size = 12.0;
        let scale = font_size * scale_factor;

        let scale_font = font.as_scaled(PxScale { x: scale, y: scale }); // y scale has not impact
        let width = scale_font.h_advance(font.glyph_id('W'));
        let height = scale_font.height();
        let line_gap = scale_font.line_gap();
        let char_width = width;
        let line_height = height;
        println!("char height: {}, width {}, gap: {}", height, width, line_gap);
        Self {
            lines: split,
            glyph_brush_text_editor: glyph_brush,
            glyph_brush_line_number,
            scroll_offset: Default::default(),
            lines_count,
            char_width,
            line_height,
            scale,
            gutter_width: 0.0,
            has_pressed_arrow_key: false,
            text_editor_viewport: Rect { min: Pos2::default(), max: Pos2::default() },
            cursor_index: Default::default(),
            cursor_pos: Default::default(),
            start_dragged_index: Default::default(),
            stop_dragged_index: Default::default(),
            selection_start_index: Default::default(),
            selection_end_index: Default::default(),
            highlighted_word: None,
            word_occurrences: RefCell::new(vec![]),
            opening_char: RefCell::new(None),
            closing_char: RefCell::new(None),
            opening_char_index: RefCell::new(None),
            closing_char_index: RefCell::new(None),
            unsaved_stated: None,
            history: vec![],
            history_index: 0,
            latest_change_time: 0.0,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.lines.len() == 0 {
            self.lines.push(String::default());
        }
        self.lines_count = self.lines.len();

        // We implement a virtual scroll, the viewport rect is static.
        let viewport = ui.max_rect();
        // Gutter display line numbers
        self.gutter_width = (TextEditor::count_digit(self.lines_count).max(1) as f32 * self.char_width);
        // Gutter
        let gutter_rect = Rect { min: Pos2 { x: viewport.min.x, y: viewport.min.y }, max: Pos2 { x: viewport.min.x + self.gutter_width, y: viewport.max.y } };

        // We declare a scroll area to have a scroll and use it scroll offset to display our data.
        let mut scroll_area = egui::ScrollArea::both();

        self.text_editor_viewport = viewport;
        self.text_editor_viewport.min.x = gutter_rect.max.x;
        let text_editor_viewport_height = (self.text_editor_viewport.max.y - self.text_editor_viewport.min.y);
        let text_editor_viewport_width = (self.text_editor_viewport.max.x - self.text_editor_viewport.min.x);
        let max_lines = (text_editor_viewport_height / self.line_height);
        let first_line_index = self.first_line_index();
        let last_line_index = self.last_line_Index(max_lines, first_line_index);

        if self.has_pressed_arrow_key {
            self.has_pressed_arrow_key = false;
            // when cursor is not more visible in the viewport, we want to scroll to it
            let cursor_offset_y = self.scroll_offset.y + text_editor_viewport_height - self.line_height - self.cursor_pos.y;
            if cursor_offset_y < 0.0 {
                let mut hidden_lines: f32 = ((cursor_offset_y.abs() / self.line_height) as usize + 1) as f32;
                self.scroll_offset.y += hidden_lines * self.line_height;
                scroll_area = scroll_area.vertical_scroll_offset(self.scroll_offset.y);
            } else if cursor_offset_y > text_editor_viewport_height {
                let mut hidden_lines: f32 = (((cursor_offset_y - text_editor_viewport_height) / self.line_height) as usize + 1) as f32;
                self.scroll_offset.y -= hidden_lines * self.line_height;
                scroll_area = scroll_area.vertical_scroll_offset(self.scroll_offset.y);
            }
            if self.cursor_pos.x - self.text_editor_viewport.min.x < 0.0 {
                self.scroll_offset.x = self.scroll_offset.x + self.cursor_pos.x - self.text_editor_viewport.min.x - self.char_width;
                scroll_area = scroll_area.horizontal_scroll_offset(self.scroll_offset.x);
            } else if self.cursor_pos.x + self.scroll_offset.x > text_editor_viewport_width + self.text_editor_viewport.min.x + self.scroll_offset.x - (2.0 * self.char_width) {
                self.scroll_offset.x = self.scroll_offset.x + self.char_width;
                scroll_area = scroll_area.horizontal_scroll_offset(self.scroll_offset.x);
            }
        }
        if self.scroll_offset.y > self.lines_count as f32 * self.line_height {
            self.scroll_offset.y = self.lines_count as f32 * self.line_height;
        }
        if self.cursor_index.x == 0 && self.cursor_pos.x != self.text_editor_viewport.min.x {
            self.cursor_pos.x = self.text_editor_viewport.min.x;
        }
        // Gutter
        self.gutter(ui, gutter_rect, first_line_index, last_line_index);

        let output = scroll_area.show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_height(self.lines_count as f32 * self.line_height);
            ui.add(
                |ui: &mut Ui| -> Response {
                    let id = ui.allocate_ui_at_rect(self.text_editor_viewport, |viewport_ui| viewport_ui.id()).inner;
                    let mut shapes = vec![];
                    let mut text_line = vec![];
                    let mut max_char_count = 0;
                    let mut opening_char_occurrence = 0;
                    self.word_occurrences = RefCell::new(vec![]);
                    opening_char_occurrence = self.find_opening_matching_char(first_line_index, last_line_index, opening_char_occurrence);


                    for (relative_line_index, frag) in self.lines[first_line_index..last_line_index].iter().enumerate() {
                        let absolute_line_index = relative_line_index + first_line_index;
                        self.highlight_word_occurrences(frag, absolute_line_index);

                        self.paint_debug_char(self.text_editor_viewport.min.y, &mut shapes, relative_line_index, absolute_line_index, frag);
                        opening_char_occurrence = self.find_closing_matching_char(opening_char_occurrence, frag, absolute_line_index);
                        if max_char_count < frag.len() {
                            max_char_count = frag.len();
                        }
                        text_line.push(format!("{}\n", frag));
                    }

                    let mut brush_mut = self.glyph_brush_text_editor.as_ref().lock().unwrap();
                    let section = glow_glyph::Section {
                        screen_position: (0.0 - self.scroll_offset.x, 0.0),
                        text: text_line.iter().map(|line| {
                            Text::default().with_text(&line).with_color([0.0, 0.0, 0.0, 1.0]).with_scale(self.scale)
                        }).collect::<Vec<Text>>(),
                        layout: glow_glyph::Layout::default_wrap(),
                        ..Section::default()
                    };
                    brush_mut.queue(section);
                    drop(brush_mut);

                    // Paint text selection
                    shapes.extend(self.selection_shapes(first_line_index));
                    // Paint cursor
                    if self.cursor_index.y >= first_line_index {
                        shapes.push(self.cursor_shape(first_line_index));
                    }
                    // Paint matching {},[],() highlight
                    self.paint_matching_opening_closing_char(first_line_index, &mut shapes);

                    self.paint_word_occurrences(first_line_index, &mut shapes);

                    ui.painter().extend(shapes);

                    // Paint text
                    let mut glyph_brush = self.glyph_brush_text_editor.clone();
                    ui.painter().add(egui::epaint::PaintCallback {
                        rect: self.text_editor_viewport,
                        callback:  std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                            let mut brush_mut = glyph_brush.lock().unwrap();
                            brush_mut.draw_queued(&painter.gl(),
                                                  (text_editor_viewport_width) as u32, (text_editor_viewport_height) as u32)
                                .expect("Draw queued");
                        })),
                    });

                    let response = ui.interact(self.text_editor_viewport, id, Sense::click_and_drag());
                    ui.memory_mut(|mem| mem.request_focus(id));
                    if response.hovered() {
                        ui.output_mut(|mem| mem.cursor_icon = CursorIcon::Text);
                    }
                    if response.clicked() {
                        self.on_click(ui);
                        response.request_focus();
                    }
                    if response.double_clicked() {
                        self.on_double_click(ui);
                    }
                    if response.drag_started() {
                        self.on_drag_start(ui);
                    }
                    if response.dragged() {
                        self.on_drag(ui);
                    }
                    self.handle_key_events(&ui, &ui.input(|input| input.events.clone()).as_ref());
                    ui.set_min_width(self.gutter_width + (self.char_width) * max_char_count as f32);
                    response
                },
            );
        });
        self.scroll_offset.y = output.state.offset.y;
        if output.state.offset.x != self.scroll_offset.x {
            self.scroll_offset.x = output.state.offset.x;
            self.set_cursor_x(self.cursor_index.x);
        }

        self.feed_history(ui);
    }

    fn find_closing_matching_char(&self, mut opening_char_occurrence: i32, frag: &String, absolute_line_index: usize) -> i32 {
        if self.opening_char.borrow().is_some() && self.closing_char.borrow().is_none() {
            let opening_char_index_ref = self.opening_char_index.borrow();
            let opening_char = self.opening_char.borrow().unwrap();
            let opening_char_index = opening_char_index_ref.as_ref().unwrap();
            if absolute_line_index >= opening_char_index.y {
                for (i, c) in frag.chars().enumerate() {
                    if opening_char_index.y == absolute_line_index && i < opening_char_index.x {
                        continue;
                    };
                    if c == opening_char {
                        opening_char_occurrence += 1;
                    } else if Self::matching_closing_char(opening_char) == c {
                        opening_char_occurrence -= 1;
                    }
                    if Self::matching_closing_char(opening_char) == c && opening_char_occurrence == 0 {
                        *self.closing_char.borrow_mut() = Some(c);
                        *self.closing_char_index.borrow_mut() = Some(Pos {
                            x: i + 1,
                            y: absolute_line_index,
                        });
                        break;
                    }
                };
            }
        }
        opening_char_occurrence
    }

    fn find_opening_matching_char(&mut self, first_line_index: usize, last_line_index: usize, mut opening_char_occurrence: i32) -> i32 {
        let should_find_opening = self.closing_char.borrow().is_some() && self.opening_char.borrow().is_none();
        if should_find_opening {
            for (relative_line_index, frag) in self.lines[first_line_index..(last_line_index + 1).min(self.lines.len())].iter().rev().enumerate() {
                let absolute_line_index = last_line_index - relative_line_index;
                if self.closing_char.borrow().is_some() && self.opening_char.borrow().is_none() {
                    let closing_char_index_ref = self.closing_char_index.borrow();
                    let closing_char_index = closing_char_index_ref.as_ref().unwrap();
                    if absolute_line_index <= closing_char_index.y {
                        for (i, c) in frag.chars().rev().enumerate() {
                            if closing_char_index.y == absolute_line_index && frag.len() - i > closing_char_index.x {
                                continue;
                            }
                            if c == self.closing_char.borrow().unwrap() {
                                opening_char_occurrence += 1;
                            } else if Self::matching_opening_char(self.closing_char.borrow().unwrap()) == c {
                                opening_char_occurrence -= 1;
                            }
                            if Self::matching_opening_char(self.closing_char.borrow().unwrap()) == c && opening_char_occurrence == 0 {
                                *self.opening_char.borrow_mut() = Some(c);
                                *self.opening_char_index.borrow_mut() = Some(Pos {
                                    x: frag.len() - i - 1,
                                    y: absolute_line_index,
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }
        opening_char_occurrence
    }

    fn highlight_word_occurrences(&self, frag: &String, absolute_line_index: usize) {
        let mut should_search_word = false;
        let mut word_char_search_index = 0;
        let mut start_index = 0;
        if self.highlighted_word.is_some() {
            let highlighted_word = self.highlighted_word.as_ref().unwrap();
            for (i, c) in frag.chars().enumerate() {
                if Self::is_char_non_part_of_word(c) {
                    if word_char_search_index == highlighted_word.len() {
                        self.word_occurrences.borrow_mut().push((Pos { x: start_index + 1, y: absolute_line_index }, Pos { x: i, y: absolute_line_index }));
                    }
                    should_search_word = true;
                    word_char_search_index = 0;
                    start_index = i;
                    continue;
                }
                if should_search_word {
                    let word_char = highlighted_word.chars().nth(word_char_search_index);
                    if word_char.is_some() && word_char.unwrap() == c {
                        word_char_search_index += 1;
                    } else {
                        word_char_search_index = 0;
                        should_search_word = false;
                    }
                }
            }
            if word_char_search_index == highlighted_word.len() {
                self.word_occurrences.borrow_mut().push((Pos { x: start_index + 1, y: absolute_line_index }, Pos { x: frag.chars().count(), y: absolute_line_index }));
            }
        }
    }

    fn on_drag(&mut self, ui: &mut Ui) {
        let maybe_pos = ui.input(|input| input.pointer.interact_pos());
        let cursor_pos = maybe_pos.unwrap();
        self.stop_dragged_index = Some(Pos::<usize> { x: self.x_to_index(cursor_pos.x - self.line_x_offset()), y: self.y_to_index(cursor_pos.y - self.text_editor_viewport.min.y) });
        self.set_selection();
        self.set_cursor_x(self.x_to_index(cursor_pos.x - (self.line_x_offset())));
        self.set_cursor_y(self.y_to_index(cursor_pos.y - self.text_editor_viewport.min.y));
    }

    fn on_drag_start(&mut self, ui: &mut Ui) {
        let maybe_pos = ui.input(|input| input.pointer.interact_pos());
        let cursor_pos = maybe_pos.unwrap();
        self.start_dragged_index = Some(Pos::<usize> { x: self.x_to_index(cursor_pos.x - self.line_x_offset()), y: self.y_to_index(cursor_pos.y - self.text_editor_viewport.min.y) });
        self.stop_dragged_index = None;
    }

    fn on_click(&mut self, ui: &mut Ui) {
        let maybe_pos = ui.input(|input| input.pointer.interact_pos());
        if maybe_pos.is_some() {
            let cursor_pos = maybe_pos.unwrap();
            self.set_cursor_x(self.x_to_index(cursor_pos.x - (self.line_x_offset())));
            self.set_cursor_y(self.y_to_index(cursor_pos.y - self.text_editor_viewport.min.y));
            self.reset_selection();
        }
    }

    fn on_double_click(&mut self, ui: &mut Ui) {
        let maybe_pos = ui.input(|input| input.pointer.interact_pos());
        if maybe_pos.is_some() {
            let cursor_pos = maybe_pos.unwrap();
            let y_index = self.y_to_index(cursor_pos.y - self.text_editor_viewport.min.y);
            let line = self.lines[y_index].as_str();
            let x_index = self.x_to_index(cursor_pos.x - (self.line_x_offset()));
            let mut start_index = 0 as usize;
            let mut end_index = 0 as usize;
            for (i, c) in line.chars().enumerate() {
                if Self::is_char_non_part_of_word(c) {
                    if i >= x_index {
                        end_index = i;
                        break;
                    } else {
                        start_index = i + 1;
                    }
                }
            }
            if end_index == 0 {
                end_index = line.len();
            }
            self.selection_start_index = Some(Pos { x: start_index, y: y_index });
            self.selection_end_index = Some(Pos { x: end_index, y: y_index });
            if end_index - start_index > 1 {
                self.highlighted_word = Some((&line[start_index..end_index]).to_string());
            }
            self.set_cursor_x(end_index);
        }
    }

    fn is_char_non_part_of_word(c: char) -> bool {
        !c.is_alphanumeric() && c != '_' && c != '-'
    }

    fn handle_key_events(&mut self, ui: &Ui, events: &Vec<Event>) {
        for event in events {
            match event {
                Event::Key { key, pressed: true, modifiers, .. } => self.on_key_press(ui, *key, modifiers),
                Event::Text(text_to_insert) => {
                    if self.has_selection() {
                        self.key_press_on_selection(Some(text_to_insert));
                    } else {
                        self.insert_text_at(text_to_insert, self.cursor_index.clone());
                    }
                    self.push_action_to_unsaved_state(&ui, SingleAction::AddChar(AddCharAction { start_pos: self.cursor_index.clone(), char: text_to_insert.clone() }
                    ));
                    self.set_cursor_x(self.cursor_index.x + 1);
                }
                _ => {}
            }
        }
    }


    fn first_line_index(&self) -> usize {
        let mut first_line_index = (self.scroll_offset.y / self.line_height) as usize;

        if first_line_index > self.lines.len() - 1 && self.lines.len() > 1 {
            first_line_index = self.lines.len() - 2;
        } else if first_line_index > self.lines.len() {
            first_line_index = self.lines.len() - 1;
        }
        first_line_index
    }

    fn last_line_Index(&self, max_lines: f32, first_line_index: usize) -> usize {
        let mut last_line_index = first_line_index as usize + max_lines as usize;
        if last_line_index > self.lines.len() {
            last_line_index = self.lines.len();
        }
        last_line_index
    }

    fn on_key_press(&mut self, ui: &Ui, key: Key, modifiers: &Modifiers) {
        match key {
            Key::ArrowDown | Key::ArrowUp => {
                if modifiers.shift {
                    if self.start_dragged_index.is_none() {
                        self.start_dragged_index = Some(self.cursor_index.clone());
                    }
                } else {
                    self.reset_selection();
                }
                self.has_pressed_arrow_key = true;
                if key == Key::ArrowDown {
                    self.set_cursor_y(self.cursor_index.y + 1);
                } else if self.cursor_index.y > 0 {
                    self.set_cursor_y(self.cursor_index.y - 1);
                }
                if modifiers.shift {
                    self.stop_dragged_index = Some(self.cursor_index.clone());
                    self.set_selection();
                }
            }
            Key::ArrowLeft | Key::ArrowRight => {
                if modifiers.shift {
                    if self.start_dragged_index.is_none() {
                        self.start_dragged_index = Some(self.cursor_index.clone());
                    }
                } else {
                    self.reset_selection();
                }
                self.has_pressed_arrow_key = true;
                if key == Key::ArrowRight {
                    self.set_cursor_x(self.cursor_index.x + 1);
                } else if self.cursor_index.x > 0 {
                    self.set_cursor_x(self.cursor_index.x - 1);
                }
                if modifiers.shift {
                    self.stop_dragged_index = Some(self.cursor_index.clone());
                    self.set_selection();
                }
            }
            Key::Backspace => {
                let line = &self.lines[self.cursor_index.y];
                let line_len = line.len();
                if self.has_selection() {
                    self.key_press_on_selection(None);
                    return;
                } else if line_len > 0 && self.cursor_index.x > 0 {
                    self.push_action_to_unsaved_state(ui, SingleAction::RemoveChar(RemoveCharAction {
                        start_pos: self.cursor_index.clone(),
                        char: self.lines[self.cursor_index.y].chars().nth(self.cursor_index.x - 1).unwrap(),
                    }));
                    self.set_cursor_x(self.cursor_index.x - 1);
                    self.remove_char_at(self.cursor_index.clone());
                } else if self.cursor_index.x == 0 && self.cursor_index.y > 0 {
                    self.push_action_to_unsaved_state(ui, SingleAction::RemoveLine(self.cursor_index.y));
                    let previous_line_len = self.lines[self.cursor_index.y - 1].len();
                    let line = self.lines.remove(self.cursor_index.y);
                    if !line.is_empty() {
                        self.lines[self.cursor_index.y - 1].push_str(line.as_str());
                    }
                    self.set_cursor_y(self.cursor_index.y - 1);
                    self.set_cursor_x(previous_line_len);
                }
            }
            Key::Delete => {
                let line = &self.lines[self.cursor_index.y];
                let line_len = line.len();
                let x_index = line.byte_index_from_char_index(self.cursor_index.x);
                if self.has_selection() {
                    self.key_press_on_selection(None);
                    return;
                } else if line_len > x_index {
                    self.push_action_to_unsaved_state(ui, SingleAction::RemoveChar(RemoveCharAction {
                        start_pos: self.cursor_index.clone(),
                        char: self.lines[self.cursor_index.y].chars().nth(self.cursor_index.x).unwrap(),
                    }));
                    self.remove_char_at(self.cursor_index.clone());
                } else if line_len == 0 && self.cursor_index.y + 1 < self.lines.len() {
                    self.push_action_to_unsaved_state(ui, SingleAction::RemoveLine(self.cursor_index.y));
                    self.lines.remove(self.cursor_index.y);
                    self.set_cursor_y(self.cursor_index.y);
                } else if line_len == x_index && self.cursor_index.y + 1 < self.lines.len() {
                    self.push_action_to_unsaved_state(ui, SingleAction::RemoveLine(self.cursor_index.y + 1));
                    let mut line = self.lines.remove(self.cursor_index.y + 1);
                    if !line.is_empty() {
                        self.lines[self.cursor_index.y].push_str(line.as_str());
                    }
                }
            }
            Key::Enter => {
                if self.has_selection() {
                    self.key_press_on_selection(None);
                    return;
                }
                self.has_pressed_arrow_key = true;
                let line = &self.lines[self.cursor_index.y].clone();
                let line_len = line.len();
                let x_index = line.byte_index_from_char_index(self.cursor_index.x);
                let line_start = &line[0..x_index];
                let line_end = &line[x_index..line_len];
                self.lines[self.cursor_index.y] = line_start.to_string();
                self.lines.insert(self.cursor_index.y + 1, line_end.to_string());
                self.push_action_to_unsaved_state(ui, SingleAction::NewLine(self.cursor_index.clone()));
                self.set_cursor_y(self.cursor_index.y + 1);
                self.set_cursor_x(0);
            }
            Key::A => {
                if modifiers.ctrl { // TODO check for mac
                    let y_index = self.lines.len() - 1;
                    let last_line = &self.lines[y_index];
                    self.start_dragged_index = Some(Pos { x: 0, y: 0 });
                    self.stop_dragged_index = Some(Pos { x: last_line.chars().count(), y: y_index });
                    self.set_selection();
                }
            }
            Key::S => {
                if modifiers.ctrl { // TODO check for mac
                    println!("ctr + s");
                }
            }
            Key::Z => {
                if modifiers.ctrl { // TODO check for mac
                    println!("ctr + z");
                    let maybe_state = self.history.pop();
                    if maybe_state.is_some() {
                        let state = maybe_state.unwrap();
                        match state.bulk_action {
                            BulkAction::AddText(action) => {
                                self.lines.splice(action.start_index..self.lines.len().min(action.end_index + 1), action.lines);
                            }
                            BulkAction::RemoveText(action) => {
                                let start = self.lines[0..action.start_index].to_vec();
                                let mut end = vec![];
                                if action.end_index + 1 <= self.lines.len() - 1 {
                                    end = self.lines[action.end_index + 1..self.lines.len()].to_vec();
                                }
                                self.lines = [start, action.lines, end].concat();
                            }
                        }
                        self.cursor_pos = state.cursor_pos;
                        self.cursor_index = state.cursor_index;
                    }
                }
            }
            _ => {}
        }
    }

    fn feed_history(&mut self, ui: &Ui) {
        let maybe_state = self.flush_unsaved_state(ui.input(|input| input.time));
        if maybe_state.is_some() {
            self.history.push(maybe_state.unwrap());
        }
    }

    fn remove_char_at(&mut self, pos: Pos<usize>) {
        self.lines[pos.y].delete_char_range(pos.x..pos.x + 1)
    }

    fn insert_text_at(&mut self, text_to_insert: &String, pos: Pos<usize>) {
        self.lines[pos.y].insert_text(text_to_insert, pos.x);
    }

    fn after_cursor_position_change(&mut self) {
        if self.cursor_index.x == 0 {
            *self.opening_char_index.borrow_mut() = None;
            *self.opening_char.borrow_mut() = None;
            *self.closing_char.borrow_mut() = None;
            *self.closing_char_index.borrow_mut() = None;
            return;
        }
        let maybe_char = self.lines[self.cursor_index.y].chars().nth(self.cursor_index.x - 1);
        if maybe_char.is_some() {
            if maybe_char.unwrap() == '{' || maybe_char.unwrap() == '(' || maybe_char.unwrap() == '[' {
                let mut index = self.cursor_index.clone();
                index.x = index.x - 1;
                *self.opening_char.borrow_mut() = maybe_char;
                *self.opening_char_index.borrow_mut() = Some(index);
                *self.closing_char.borrow_mut() = None;
                *self.closing_char_index.borrow_mut() = None;
                return;
            } else if maybe_char.unwrap() == '}' || maybe_char.unwrap() == ')' || maybe_char.unwrap() == ']' {
                let mut index = self.cursor_index.clone();
                index.x = index.x;
                *self.opening_char.borrow_mut() = None;
                *self.opening_char_index.borrow_mut() = None;
                *self.closing_char.borrow_mut() = maybe_char;
                *self.closing_char_index.borrow_mut() = Some(index);
                return;
            }
        }
        *self.opening_char.borrow_mut() = None;
        *self.opening_char_index.borrow_mut() = None;
        *self.closing_char.borrow_mut() = None;
        *self.closing_char_index.borrow_mut() = None;
    }

    #[inline]
    fn sanitize_cursor_position(&mut self) {
        if self.cursor_index.y >= self.lines_count {
            self.set_cursor_y(self.lines_count - 1);
        }
        let line = &self.lines[self.cursor_index.y];
        let line_len = line.len();
        if self.cursor_index.x > line_len {
            self.set_cursor_x(line_len);
        }
    }

    #[inline]
    fn line_index_from_line_y(&self, line_y: f32) -> usize {
        // line_y is from the virtual scroll rect, need to add the scroll offset y to get the actual position.
        ((line_y + self.scroll_offset.y) / self.line_height) as usize
    }

    #[inline]
    fn y_to_index(&self, y: f32) -> usize {
        // convert y to line_number
        // e.g: line_height = 10; (thus: line min.y = 10, line max.y = 20)
        // if y = 15 then line_number = 1 + 1
        let line_number = ((y / self.line_height) as usize) + 1;
        self.line_index_from_line_y(line_number as f32 * self.line_height) - 1
    }

    #[inline]
    fn x_to_index(&self, x: f32) -> usize {
        ((x) / self.char_width) as usize
    }

    #[inline]
    fn line_at(&self, y: f32) -> &str {
        self.lines[self.y_to_index(y)].as_str()
    }

    #[inline]
    fn index_to_pos(&self, index: Pos<usize>) -> Pos<f32> {
        Pos::<f32> {
            x: self.index_to_x(index.x),
            y: self.index_to_y(index.y),
        }
    }

    #[inline]
    fn index_to_y(&self, index: usize) -> f32 {
        index as f32 * self.line_height
    }

    #[inline]
    fn index_to_y_in_virtual_scroll(&self, index: usize, first_visible_index: usize) -> f32 {
        // caller need to ensure that index is greater than first_visible_index
        (index - first_visible_index) as f32 * self.line_height
    }

    #[inline]
    fn index_to_x(&self, index: usize) -> f32 {
        index as f32 * self.char_width + (self.line_x_offset())
    }

    #[inline]
    fn set_cursor_y(&mut self, new_value: usize) {
        if self.cursor_index.y == new_value {
            return;
        }
        self.cursor_index.y = new_value;
        self.cursor_pos.y = self.index_to_y(self.cursor_index.y);
        self.sanitize_cursor_position();
        self.after_cursor_position_change();
    }

    #[inline]
    fn set_cursor_x(&mut self, new_value: usize) {
        if self.cursor_index.x == new_value {
            return;
        }
        self.cursor_index.x = new_value;
        self.cursor_pos.x = self.index_to_x(self.cursor_index.x);
        self.sanitize_cursor_position();
        self.after_cursor_position_change();
    }

    #[inline]
    fn line_x_offset(&self) -> f32 {
        self.text_editor_viewport.min.x - self.scroll_offset.x / 2.0
    }

    fn matching_closing_char(opening: char) -> char {
        match opening {
            '{' => '}',
            '(' => ')',
            '[' => ']',
            _ => opening
        }
    }

    fn matching_opening_char(closing: char) -> char {
        match closing {
            '}' => '{',
            ')' => '(',
            ']' => '[',
            _ => closing
        }
    }

    fn count_digit(number: usize) -> usize {
        if number >= 100_000_000 {
            9
        } else if number >= 100_000_00 {
            8
        } else if number >= 100_000_0 {
            7
        } else if number >= 100_000 {
            6
        } else if number >= 10_000 {
            5
        } else if number >= 1_000 {
            4
        } else if number >= 100 {
            3
        } else if number >= 10 {
            2
        } else {
            1
        }
    }

    fn paint_word_occurrences(&self, first_line_index: usize, mut shapes: &mut Vec<Shape>) {
        for (start_pos, end_pos) in self.word_occurrences.borrow().iter() {
            shapes.push(epaint::Shape::Rect(RectShape {
                rect: Rect {
                    min: Pos2 { x: self.index_to_x(start_pos.x) as f32, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(start_pos.y, first_line_index) },
                    max: Pos2 { x: self.index_to_x(end_pos.x) as f32, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(start_pos.y, first_line_index) + self.line_height },
                },
                rounding: Rounding::none(),
                fill: Color32::YELLOW,
                stroke: Default::default(),
            }));
        }
    }

    fn paint_matching_opening_closing_char(&self, first_line_index: usize, mut shapes: &mut Vec<Shape>) {
        if self.opening_char_index.borrow().is_some() {
            let opening_char_index_ref = self.opening_char_index.borrow();
            let opening_char_index = opening_char_index_ref.as_ref().unwrap();
            if opening_char_index.y >= first_line_index {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.index_to_x(opening_char_index.x) as f32, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(opening_char_index.y, first_line_index) },
                        max: Pos2 { x: self.index_to_x(opening_char_index.x) as f32 + self.char_width, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(opening_char_index.y, first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::GREEN,
                    stroke: Default::default(),
                }));
            }
        }
        if self.closing_char_index.borrow().is_some() {
            let closing_char_index_ref = self.closing_char_index.borrow();
            let closing_char_index = closing_char_index_ref.as_ref().unwrap();
            if closing_char_index.y >= first_line_index && closing_char_index.x > 0 {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.index_to_x(closing_char_index.x - 1) as f32, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(closing_char_index.y, first_line_index) },
                        max: Pos2 { x: self.index_to_x(closing_char_index.x - 1) as f32 + self.char_width, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(closing_char_index.y, first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::GREEN,
                    stroke: Default::default(),
                }));
            }
        }
    }

    fn paint_line_number(&self, line_y_offset: f32, line_number: usize) {
        let mut brush_mut = self.glyph_brush_line_number.as_ref().lock().unwrap();
        let mut color = [0.0, 0.0, 0.0, 1.0];
        if line_number - 1 == self.cursor_index.y {
            color = [1.0, 0.0, 0.0, 1.0];
        }
        brush_mut.queue(glow_glyph::Section {
            screen_position: (0.0, line_y_offset),
            text: vec![Text::default()
                .with_text(line_number.to_string().as_str())
                .with_color(color)
                .with_scale(self.scale)],
            ..Section::default()
        });
    }

    fn paint_debug_char(&self, top: f32, mut shapes: &mut Vec<Shape>, i: usize, line_number: usize, frag: &String) {
        if line_number == self.cursor_index.y {
            for j in 0..frag.len() {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: emath::Rect {
                        min: Pos2 { x: self.text_editor_viewport.min.x + j as f32 * self.char_width, y: top + (self.line_height) * (i) as f32 },
                        max: Pos2 { x: self.text_editor_viewport.min.x + (j + 1) as f32 * self.char_width, y: top + (self.line_height) * (i + 1) as f32 },
                    }
                    ,
                    fill: if j % 2 == 0 {
                        Color32::from_rgba_premultiplied(96, 96, 96, 128)
                    } else {
                        Color32::from_rgba_premultiplied(160, 160, 160, 128)
                    },
                    stroke: Stroke::none(),
                    rounding: Default::default(),
                }));
            }
        }
    }

    fn paint_debug_line(&self, viewport: Rect, mut shapes: &mut Vec<Shape>, i: usize) {
        shapes.push(epaint::Shape::Rect(RectShape {
            rect: emath::Rect {
                min: Pos2 { x: 0.0, y: viewport.min.y + self.line_height * i as f32 },
                max: Pos2 {
                    x: viewport.max.x,
                    y: viewport.min.y + self.line_height * ((i + 1) as f32),
                },
            },
            fill: if i % 2 == 0 {
                Color32::from_rgba_premultiplied(96, 96, 96, 128)
            } else {
                Color32::from_rgba_premultiplied(160, 160, 160, 128)
            },
            stroke: Stroke::none(),
            rounding: Default::default(),
        }));
    }

    fn cursor_shape(&self, first_line_index: usize) -> Shape {
        epaint::Shape::Rect(RectShape {
            rect: Rect {
                min: Pos2 { x: self.cursor_pos.x as f32, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.cursor_index.y, first_line_index) },
                max: Pos2 { x: self.cursor_pos.x + 2.0, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.cursor_index.y, first_line_index) + self.line_height },
            },
            rounding: Rounding::none(),
            fill: Color32::RED,
            stroke: Default::default(),
        })
    }

    fn gutter(&mut self, ui: &mut Ui, gutter_rect: Rect, first_line_index: usize, last_line_index: usize) {
        let mut brush_mut = self.glyph_brush_line_number.as_ref().lock().unwrap();
        let numbers = (first_line_index..last_line_index).map(|line_number| (line_number, format!("{}\n", line_number + 1))).collect::<Vec<(usize, String)>>();
        brush_mut.queue(glow_glyph::Section {
            screen_position: (0.0, 0.0),
            text: numbers.iter().map(|(line_number, text)| {
                let mut color = [0.0, 0.0, 0.0, 1.0];
                if *line_number == self.cursor_index.y {
                    color = [1.0, 0.0, 0.0, 1.0];
                }
                Text::default().with_text(text.as_str()).with_color(color)
                    .with_scale(self.scale)
            }).collect::<Vec<Text>>(),
            ..Section::default()
        });
        drop(brush_mut);
        ui.allocate_ui_at_rect(gutter_rect, |ui| {
            ui.painter().add(epaint::Shape::Rect(RectShape {
                rect: gutter_rect,
                rounding: Rounding::none(),
                fill: Color32::LIGHT_GRAY,
                stroke: Default::default(),
            }));
            let glyph_brush = self.glyph_brush_line_number.clone();
            ui.painter().add(egui::epaint::PaintCallback {
                rect: gutter_rect,
                callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                    let mut brush_mut = glyph_brush.lock().unwrap();
                    brush_mut.draw_queued(&painter.gl(),
                                          (gutter_rect.max.x - gutter_rect.min.x) as u32, (gutter_rect.max.y - gutter_rect.min.y) as u32)
                        .expect("Draw queued");
                })),
            });
        });
    }
}

trait Selection {
    fn reset_selection(&mut self);
    fn set_selection(&mut self);
    fn has_selection(&self) -> bool;
    fn is_single_line_selection(&self) -> bool;
    fn is_two_lines_selection(&self) -> bool;
    fn selection_shapes(&self, first_line_index: usize) -> Vec<Shape>;
    fn key_press_on_selection(&mut self, text_to_insert: Option<&str>);
}

impl Selection for TextEditor {
    fn reset_selection(&mut self) {
        self.selection_start_index = None;
        self.selection_end_index = None;
        self.start_dragged_index = None;
        self.stop_dragged_index = None;
        self.highlighted_word = None;
    }
    fn set_selection(&mut self) {
        if !self.start_dragged_index.is_some() || !self.stop_dragged_index.is_some() {
            return;
        }
        let mut start_index = self.start_dragged_index.clone().unwrap();
        let mut end_index = self.stop_dragged_index.clone().unwrap();
        if self.start_dragged_index.as_ref().unwrap().y > self.stop_dragged_index.as_ref().unwrap().y { // user can drag selection from bottom to top
            start_index = self.stop_dragged_index.clone().unwrap();
            end_index = self.start_dragged_index.clone().unwrap();
        }
        if start_index.y == end_index.y && start_index.x > end_index.x { // user can drag selection from right to left
            let x = start_index.x;
            start_index.x = end_index.x;
            end_index.x = x;
        }
        if start_index.y >= self.lines_count {
            start_index.y = self.lines_count - 1;
        }
        if end_index.y >= self.lines_count {
            end_index.y = self.lines_count - 1;
        }
        let line_len = self.lines[start_index.y].len();
        if start_index.x > line_len {
            start_index.x = line_len;
        }
        let line_len = self.lines[end_index.y].len();
        if end_index.x > line_len {
            end_index.x = line_len;
        }
        self.selection_start_index = Some(start_index);
        self.selection_end_index = Some(end_index);
    }

    fn has_selection(&self) -> bool {
        return self.selection_start_index.is_some() && self.selection_end_index.is_some();
    }

    fn is_single_line_selection(&self) -> bool {
        if !self.has_selection() {
            return false;
        }
        self.selection_start_index.as_ref().unwrap().y == self.selection_end_index.as_ref().unwrap().y
    }

    fn is_two_lines_selection(&self) -> bool {
        if !self.has_selection() {
            return false;
        }
        self.selection_start_index.as_ref().unwrap().y + 1 == self.selection_end_index.as_ref().unwrap().y
    }

    fn selection_shapes(&self, first_line_index: usize) -> Vec<Shape> {
        if !self.has_selection() {
            return vec![];
        }
        if self.is_single_line_selection() { // single line selection
            if self.selection_start_index.as_ref().unwrap().y < first_line_index { // if selection is not visible
                return vec![];
            }
            vec![
                Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.index_to_x(self.selection_start_index.as_ref().unwrap().x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.as_ref().unwrap().y, first_line_index) },
                        max: Pos2 { x: self.index_to_x(self.selection_end_index.as_ref().unwrap().x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.as_ref().unwrap().y, first_line_index) + self.line_height },
                    },
                    fill: Color32::LIGHT_BLUE,
                    rounding: Rounding::none(),
                    stroke: Default::default(),
                })
            ]
        } else if self.is_two_lines_selection() { // two lines selection
            let mut shapes = vec![];
            if self.selection_start_index.as_ref().unwrap().y >= first_line_index {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.index_to_x(self.selection_start_index.as_ref().unwrap().x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.as_ref().unwrap().y, first_line_index) },
                        max: Pos2 { x: self.text_editor_viewport.max.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.as_ref().unwrap().y, first_line_index) + self.line_height },
                    },
                    fill: Color32::LIGHT_BLUE,
                    rounding: Rounding::none(),
                    stroke: Default::default(),
                }))
            }
            if self.selection_end_index.as_ref().unwrap().y >= first_line_index {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.text_editor_viewport.min.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_end_index.as_ref().unwrap().y, first_line_index) },
                        max: Pos2 { x: self.index_to_x(self.selection_end_index.as_ref().unwrap().x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_end_index.as_ref().unwrap().y, first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::LIGHT_BLUE,
                    stroke: Default::default(),
                }))
            }
            return shapes;
        } else {
            let mut shapes = vec![];
            if self.selection_start_index.as_ref().unwrap().y >= first_line_index {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.index_to_x(self.selection_start_index.as_ref().unwrap().x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.as_ref().unwrap().y, first_line_index) },
                        max: Pos2 { x: self.text_editor_viewport.max.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.as_ref().unwrap().y, first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::LIGHT_BLUE,
                    stroke: Default::default(),
                }))
            }

            if self.selection_end_index.as_ref().unwrap().y >= first_line_index {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.text_editor_viewport.min.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll((self.selection_start_index.as_ref().unwrap().y + 1).max(first_line_index), first_line_index) },
                        max: Pos2 { x: self.text_editor_viewport.max.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll((self.selection_end_index.as_ref().unwrap().y - 1).max(first_line_index), first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::LIGHT_BLUE,
                    stroke: Default::default(),
                }));
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.text_editor_viewport.min.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_end_index.as_ref().unwrap().y, first_line_index) },
                        max: Pos2 { x: self.index_to_x(self.selection_end_index.as_ref().unwrap().x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_end_index.as_ref().unwrap().y, first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::LIGHT_BLUE,
                    stroke: Default::default(),
                }))
            }
            return shapes;
        }
    }

    fn key_press_on_selection(&mut self, text_to_insert: Option<&str>) {
        let selection_start_index = self.selection_start_index.as_ref().unwrap().clone();
        let selection_end_index = self.selection_end_index.as_ref().unwrap().clone();
        if self.is_single_line_selection() {
            let line = &self.lines[selection_start_index.y];
            let line_len = line.len();
            let start_x_index = line.byte_index_from_char_index(selection_start_index.x);
            let end_x_index = line.byte_index_from_char_index(selection_end_index.x);
            self.lines[selection_start_index.y] = format!("{}{}{}", &line[0..start_x_index],
                                                          text_to_insert.unwrap_or(""),
                                                          &line[end_x_index..line_len]);
        } else if self.is_two_lines_selection() {
            let line = &self.lines[selection_start_index.y];
            let start_x_index = line.byte_index_from_char_index(selection_start_index.x);
            let new_line_start = String::from(&line[0..start_x_index]);
            self.lines.remove(selection_start_index.y);

            let line = &self.lines[selection_start_index.y];
            let line_len = line.len();
            let end_x_index = line.byte_index_from_char_index(selection_end_index.x);
            let new_line_end = String::from(&line[end_x_index..line_len]);
            self.lines[selection_start_index.y] = format!("{}{}{}", new_line_start, text_to_insert.unwrap_or(""), new_line_end);
        } else {
            let line = &self.lines[selection_start_index.y];
            let start_x_index = line.byte_index_from_char_index(selection_start_index.x);
            let new_line_start = String::from(&line[0..start_x_index]);

            let line = &self.lines[selection_end_index.y];
            let line_len = line.len();
            let end_x_index = line.byte_index_from_char_index(selection_end_index.x);
            let new_line_end = String::from(&line[end_x_index..line_len]);

            let text_start = &self.lines[0..selection_start_index.y];
            let text_end = &self.lines[selection_end_index.y..self.lines.len()];
            self.lines = [text_start, text_end].concat();
            self.lines[selection_start_index.y] = format!("{}{}{}", new_line_start, text_to_insert.unwrap_or(""), new_line_end);
        }
        self.set_cursor_y(selection_start_index.y);
        self.set_cursor_x(selection_start_index.x);
        self.reset_selection();
    }
}

trait HasUnsavedState {
    fn init_unsaved_state(&mut self, time: f64);
    fn push_action_to_unsaved_state(&mut self, ui: &Ui, action: SingleAction);
    fn flush_unsaved_state(&mut self, time: f64) -> Option<State>;
}

const InactivityPeriod: f64 = 2.0;

impl HasUnsavedState for TextEditor {
    fn init_unsaved_state(&mut self, time: f64) {
        self.unsaved_stated = Some(UnsavedState {
            last_activity_at: time,
            cursor_index: self.cursor_index.clone(),
            cursor_pos: self.cursor_pos.clone(),
            actions: vec![],
        })
    }

    fn push_action_to_unsaved_state(&mut self, ui: &Ui, action: SingleAction) {
        if self.unsaved_stated.is_none() {
            self.init_unsaved_state(ui.input(|input| input.time));
        }
        let unsaved_state = self.unsaved_stated.as_mut().unwrap();
        unsaved_state.actions.push(action);
        if ui.input(|input| input.time) - unsaved_state.last_activity_at >= InactivityPeriod {
            self.feed_history(ui);
        } else {
            unsaved_state.last_activity_at = ui.input(|input| input.time);
        }
    }

    fn flush_unsaved_state(&mut self, time: f64) -> Option<State> {
        if self.unsaved_stated.is_none() {
            return None;
        }
        let mut unsaved_state = self.unsaved_stated.as_ref().unwrap().clone();
        if time - unsaved_state.last_activity_at < InactivityPeriod {
            return None;
        }
        println!("Saving state");
        self.unsaved_stated = None;
        let mut min_index = self.lines.len();
        let mut max_index = 0;
        let mut y = 0;
        let mut added_lines = 0;
        for action in unsaved_state.actions.iter() {
            match action {
                SingleAction::AddChar(action) => y = action.start_pos.y,
                SingleAction::RemoveChar(action) => y = action.start_pos.y,
                SingleAction::RemoveLine(line_index) => y = line_index.clone(),
                SingleAction::NewLine(position) => {
                    y = position.y;
                    added_lines += 1;
                }
            }
            if min_index > y {
                min_index = y;
            }
            if max_index < y {
                max_index = y;
            }
        }
        max_index += added_lines;

        let mut lines = vec![String::default(); max_index - min_index + 1];
        lines.splice(0..lines.len(), self.lines[min_index..=(self.lines.len() - 1).min(max_index)].to_vec()).collect::<Vec<String>>();
        let before_lines_count = lines.len();
        loop {
            if unsaved_state.actions.is_empty() {
                break;
            }
            let action = unsaved_state.actions.pop().unwrap();
            match action {
                SingleAction::AddChar(action) => {
                    lines[action.start_pos.y - min_index].delete_char_range(action.start_pos.x..action.start_pos.x + 1);
                }
                SingleAction::RemoveChar(action) => {
                    lines[action.start_pos.y - min_index].insert((action.start_pos.x.max(1)) - 1, action.char);
                }
                SingleAction::RemoveLine(line_index) => {
                    lines.insert(line_index - min_index, String::default());
                }
                SingleAction::NewLine(position) => {
                    let y = position.y - min_index;
                    let start_line = lines[y].clone();
                    let end_line = lines[y + 1].clone();
                    lines[y] = format!("{}{}", start_line, end_line);
                    lines.remove(y + 1);
                }
            }
        }
        let after_lines_count = lines.len();
        let text_action = TextAction {
            start_index: min_index,
            end_index: max_index,
            lines,
        };

        Some(State {
            created_at: unsaved_state.last_activity_at,
            cursor_index: unsaved_state.cursor_index,
            cursor_pos: unsaved_state.cursor_pos,
            bulk_action: if before_lines_count <= after_lines_count {
                BulkAction::AddText(text_action)
            } else {
                BulkAction::RemoveText(text_action)
            },
        })
    }
}