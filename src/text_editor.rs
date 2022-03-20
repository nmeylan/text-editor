use std::any::Any;
use std::borrow::Borrow;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::format;
use std::default::Default;
use std::detect::__is_feature_detected::sha;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use glow_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, GlyphCruncher, Section, Text};
use eframe::emath::Vec2;
use eframe::egui::epaint::TextShape;
use eframe::egui::{Color32, Context, FontId, Galley, Pos2, Sense, TextFormat};
use eframe::egui::text::LayoutJob;
use eframe::epi::{App, Frame, Storage};
use eframe::egui::{*};
use eframe::epaint::{*};
use eframe::{egui, epi, epaint, emath};
use glow_glyph::ab_glyph::{PxScale, Font, ScaleFont};

pub struct TextEditor {
    split: Vec<String>,
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
    // Position
    cursor_index: Pos<usize>,
    cursor_pos: Pos<f32>,
    start_dragged_index: Pos<usize>,
    stop_dragged_index: Pos<usize>,
    selection_start_index: Pos<usize>,
    selection_end_index: Pos<usize>,

}

#[derive(Default, Debug, Clone)]
struct Pos<T> {
    x: T,
    y: T,
}

impl TextEditor {
    #[elapsed_time::elapsed]
    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        self.lines_count = self.split.len();

        // We implement a virtual scroll, the viewport rect is static.
        let viewport = ui.max_rect();
        // Gutter display line numbers
        self.gutter_width = (TextEditor::count_digit(self.lines_count) as f32 * (self.char_width / 2.0));
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
            } else if self.cursor_pos.x + self.scroll_offset.x > text_editor_viewport_width + self.text_editor_viewport.min.x + self.scroll_offset.x - 2.0 * self.char_width {
                self.scroll_offset.x = self.scroll_offset.x + self.char_width;
                scroll_area = scroll_area.horizontal_scroll_offset(self.scroll_offset.x);
            }
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
                    for frag in self.split[first_line_index..last_line_index].iter() {
                        if max_char_count < frag.len() {
                            max_char_count = frag.len();
                        }
                        text_line.push(format!("{}\n", frag));
                    }

                    let mut brush_mut = self.glyph_brush_text_editor.as_ref().lock().unwrap();
                    let section = glow_glyph::Section {
                        screen_position: (0.0 - self.scroll_offset.x, 0.0),
                        text: text_line.iter().map(|line| {
                            Text::default()
                                .with_text(&line)
                                .with_color([0.0, 0.0, 0.0, 1.0])
                                .with_scale(self.scale)
                        }).collect::<Vec<Text>>(),
                        layout: glow_glyph::Layout::default_wrap(),
                        ..Section::default()
                    };
                    brush_mut.queue(section);
                    drop(brush_mut);

                    shapes.extend(self.selection_shapes(first_line_index));
                    if self.cursor_index.y >= first_line_index {
                        shapes.push(self.cursor_shape(first_line_index));
                    }

                    ui.painter().extend(shapes);

                    let mut glyph_brush = self.glyph_brush_text_editor.clone();
                    ui.painter().add(egui::epaint::PaintCallback {
                        rect: self.text_editor_viewport,
                        callback: std::sync::Arc::new(move |render_ctx| {
                            if let Some(painter) = render_ctx.downcast_ref::<egui_glow::Painter>() {
                                let mut brush_mut = glyph_brush.lock().unwrap();
                                brush_mut.draw_queued(&painter.gl(),
                                                      (text_editor_viewport_width) as u32, (text_editor_viewport_height) as u32)
                                    .expect("Draw queued");
                            } else {
                                eprintln!("Can't do custom painting because we are not using a glow context");
                            }
                        }),
                    });

                    let response = ui.interact(self.text_editor_viewport, id, Sense::click_and_drag());
                    ui.memory().request_focus(id);
                    if response.hovered() {
                        ui.output().cursor_icon = CursorIcon::Text;
                    }
                    if response.clicked() {
                        let maybe_pos = ui.input().pointer.interact_pos();
                        if maybe_pos.is_some() {
                            let cursor_pos = maybe_pos.unwrap();
                            self.set_cursor_x(self.x_to_index(cursor_pos.x - (self.line_x_offset())));
                            self.set_cursor_y(self.y_to_index(cursor_pos.y - self.text_editor_viewport.min.y));
                            self.sanitize_cursor_position();
                        }
                        response.request_focus();
                    }
                    if response.drag_started() {
                        let maybe_pos = ui.input().pointer.interact_pos();
                        let cursor_pos = maybe_pos.unwrap();
                        self.start_dragged_index = Pos::<usize> { x: self.x_to_index(cursor_pos.x - self.line_x_offset()), y: self.y_to_index(cursor_pos.y - self.text_editor_viewport.min.y) };
                        self.stop_dragged_index = Pos::default();
                    }
                    if response.dragged() {
                        let maybe_pos = ui.input().pointer.interact_pos();
                        let cursor_pos = maybe_pos.unwrap();
                        self.stop_dragged_index = Pos::<usize> { x: self.x_to_index(cursor_pos.x - self.line_x_offset()), y: self.y_to_index(cursor_pos.y - self.text_editor_viewport.min.y) };
                        self.set_selection();
                        self.set_cursor_x(self.x_to_index(cursor_pos.x - (self.line_x_offset())));
                        self.set_cursor_y(self.y_to_index(cursor_pos.y - self.text_editor_viewport.min.y));
                    }
                    let events = ui.input().events.clone();
                    for event in &events {
                        match event {
                            Event::Key { key, pressed: true, .. } => self.on_key_press(*key),
                            Event::Text(text_to_insert) => {
                                self.split[self.cursor_index.y].insert_str(self.cursor_index.x, text_to_insert);
                                self.set_cursor_x(self.cursor_index.x + text_to_insert.len());
                            }
                            _ => {}
                        }
                    }
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
    }

    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let font = ab_glyph::FontArc::try_from_slice(include_bytes!(
            "Inconsolata-Regular.ttf"
        )).unwrap();

        let glyph_brush = Arc::new(Mutex::new(GlyphBrushBuilder::using_font(font.clone())
            .initial_cache_size((2048 * 2, 2048 * 2))
            .draw_cache_position_tolerance(1.0)
            .build(&creation_context.gl)));
        let glyph_brush_line_number = Arc::new(Mutex::new(GlyphBrushBuilder::using_font(font.clone())
            .initial_cache_size((120, 120))
            .draw_cache_position_tolerance(1.0)
            .build(&creation_context.gl)));

        // let content = fs::read_to_string(Path::new("/Users/nmeylan/dev/perso/meta-editor/nmeylan/src/text")).unwrap();
        let content = fs::read_to_string(Path::new("/home/nmeylan/dev/perso/rust-ragnarok-server/lib/packets/src/packets_impl.rs")).unwrap();
        let split = content.split("\n").map(|s| s.to_string()).collect::<Vec<String>>();
        let lines_count = split.len();
        let font_size = 15.0;
        let scale = font_size * 2.0;

        let scale_font = font.as_scaled(PxScale { x: scale, y: scale }); // y scale has not impact
        let width = scale_font.h_advance(font.glyph_id('W'));
        let height = scale_font.height();
        let line_gap = scale_font.line_gap();
        let char_width = width;
        let line_height = font_size;
        println!("char height: {}, width {}, gap: {}", height, width, line_gap);
        Self {
            split,
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
        }
    }

    fn first_line_index(&self) -> usize {
        let mut first_line_index = (self.scroll_offset.y / self.line_height) as usize;
        if first_line_index > self.split.len() - 1 {
            first_line_index = self.split.len() - 2;
        }
        first_line_index
    }

    fn last_line_Index(&self, max_lines: f32, first_line_index: usize) -> usize {
        let mut last_line_index = first_line_index as usize + max_lines as usize;
        if last_line_index > self.split.len() - 1 {
            last_line_index = self.split.len() - 1;
        }
        last_line_index
    }

    fn on_key_press(&mut self, key: Key) {
        match key {
            Key::ArrowDown | Key::ArrowUp => {
                self.has_pressed_arrow_key = true;
                if key == Key::ArrowDown {
                    self.set_cursor_y(self.cursor_index.y + 1);
                } else if self.cursor_index.y > 0 {
                    self.set_cursor_y(self.cursor_index.y - 1);
                }
            }
            Key::ArrowLeft | Key::ArrowRight => {
                self.has_pressed_arrow_key = true;
                if key == Key::ArrowRight {
                    self.set_cursor_x(self.cursor_index.x + 1);
                } else if self.cursor_index.x > 0 {
                    self.set_cursor_x(self.cursor_index.x - 1);
                }
            }
            Key::Backspace => {
                if self.split[self.cursor_index.y].len() > 0 && self.cursor_index.x > 0 {
                    self.split[self.cursor_index.y].remove(self.cursor_index.x - 1);
                    self.set_cursor_x(self.cursor_index.x - 1);
                } else if self.cursor_index.x == 0 {
                    let line = self.split.remove(self.cursor_index.y);
                    if !line.is_empty() {
                        self.split[self.cursor_index.y - 1].push_str(line.as_str());
                    }
                    self.set_cursor_y(self.cursor_index.y - 1);
                    self.set_cursor_x(self.split[self.cursor_index.y].len());
                }
            }
            Key::Delete => {
                if self.split[self.cursor_index.y].len() > self.cursor_index.x {
                    self.split[self.cursor_index.y].remove(self.cursor_index.x);
                } else if self.split[self.cursor_index.y].len() == 0 {
                    self.split.remove(self.cursor_index.y);
                    self.set_cursor_y(self.cursor_index.y);
                } else if self.split[self.cursor_index.y].len() == self.cursor_index.x && self.cursor_index.y + 1 < self.split.len() {
                    let line = self.split.remove(self.cursor_index.y + 1);
                    if !line.is_empty() {
                        self.split[self.cursor_index.y].push_str(line.as_str());
                    }
                }
            }
            Key::Enter => {
                self.split.insert(self.cursor_index.y + 1, String::default());
                self.set_cursor_y(self.cursor_index.y + 1);
            }
            _ => {}
        }
    }

    #[inline]
    fn sanitize_cursor_position(&mut self) {
        if self.cursor_index.y >= self.lines_count {
            self.set_cursor_y(self.lines_count - 1);
        }
        let line_len = self.split[self.cursor_index.y].len();
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
        ((x) / (self.char_width / 2.0)) as usize
    }

    #[inline]
    fn line_at(&self, y: f32) -> &str {
        self.split[self.y_to_index(y)].as_str()
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
        index as f32 * (self.char_width / 2.0) + (self.line_x_offset())
    }

    #[inline]
    fn set_cursor_y(&mut self, new_value: usize) {
        self.cursor_index.y = new_value;
        self.cursor_pos.y = self.index_to_y(self.cursor_index.y);
        self.sanitize_cursor_position();
    }

    #[inline]
    fn set_cursor_x(&mut self, new_value: usize) {
        self.cursor_index.x = new_value;
        self.cursor_pos.x = self.index_to_x(self.cursor_index.x);
        self.sanitize_cursor_position();
    }

    #[inline]
    fn line_x_offset(&self) -> f32 {
        self.text_editor_viewport.min.x - self.scroll_offset.x / 2.0
    }

    fn set_selection(&mut self) {
        let mut start_index = self.start_dragged_index.clone();
        let mut end_index = self.stop_dragged_index.clone();
        if self.start_dragged_index.y > self.stop_dragged_index.y { // user can drag selection from bottom to top
            start_index = self.stop_dragged_index.clone();
            end_index = self.start_dragged_index.clone();
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
        let line_len = self.split[start_index.y].len();
        if start_index.x > line_len {
            start_index.x = line_len;
        }
        let line_len = self.split[end_index.y].len();
        if end_index.x > line_len {
            end_index.x = line_len;
        }
        self.selection_start_index = start_index;
        self.selection_end_index = end_index;
    }

    fn is_single_line_selection(&self) -> bool {
        self.selection_start_index.y == self.selection_end_index.y
    }

    fn is_two_lines_selection(&self) -> bool {
        self.selection_start_index.y + 1 == self.selection_end_index.y
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
        if line_number - 1 == self.cursor_index.y {
            for j in 0..frag.len() {
                // if j as f32 * self.char_width < gutter_width {
                //     continue;
                // }
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: emath::Rect {
                        min: Pos2 { x: self.text_editor_viewport.min.x + j as f32 * self.char_width / 2.0, y: top + (self.line_height) * (i) as f32 },
                        max: Pos2 { x: self.text_editor_viewport.min.x + (j + 1) as f32 * self.char_width / 2.0, y: top + (self.line_height) * (i + 1) as f32 },
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

    fn selection_shapes(&self, first_line_index: usize) -> Vec<Shape> {
        if self.is_single_line_selection() { // single line selection
            if self.selection_start_index.y < first_line_index { // if selection is not visible
                return vec![];
            }
            vec![
                epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.index_to_x(self.selection_start_index.x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.y, first_line_index) },
                        max: Pos2 { x: self.index_to_x(self.selection_end_index.x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.y, first_line_index) + self.line_height },
                    },
                    fill: Color32::LIGHT_BLUE,
                    rounding: Rounding::none(),
                    stroke: Default::default(),
                })
            ]
        } else if self.is_two_lines_selection() { // two lines selection
            let mut shapes = vec![];
            if self.selection_start_index.y >= first_line_index {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.index_to_x(self.selection_start_index.x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.y, first_line_index) },
                        max: Pos2 { x: self.text_editor_viewport.max.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.y, first_line_index) + self.line_height },
                    },
                    fill: Color32::LIGHT_BLUE,
                    rounding: Rounding::none(),
                    stroke: Default::default(),
                }))
            }
            if self.selection_end_index.y >= first_line_index {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.text_editor_viewport.min.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_end_index.y, first_line_index) },
                        max: Pos2 { x: self.index_to_x(self.selection_end_index.x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_end_index.y, first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::LIGHT_BLUE,
                    stroke: Default::default(),
                }))
            }
            return shapes;
        } else {
            let mut shapes = vec![];
            if self.selection_start_index.y >= first_line_index {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.index_to_x(self.selection_start_index.x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.y, first_line_index) },
                        max: Pos2 { x: self.text_editor_viewport.max.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_start_index.y, first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::LIGHT_BLUE,
                    stroke: Default::default(),
                }))
            }

            if self.selection_end_index.y >= first_line_index {
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.text_editor_viewport.min.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll((self.selection_start_index.y + 1).max(first_line_index), first_line_index) },
                        max: Pos2 { x: self.text_editor_viewport.max.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll((self.selection_end_index.y - 1).max(first_line_index), first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::LIGHT_BLUE,
                    stroke: Default::default(),
                }));
                shapes.push(epaint::Shape::Rect(RectShape {
                    rect: Rect {
                        min: Pos2 { x: self.text_editor_viewport.min.x, y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_end_index.y, first_line_index) },
                        max: Pos2 { x: self.index_to_x(self.selection_end_index.x), y: self.text_editor_viewport.min.y + self.index_to_y_in_virtual_scroll(self.selection_end_index.y, first_line_index) + self.line_height },
                    },
                    rounding: Rounding::none(),
                    fill: Color32::LIGHT_BLUE,
                    stroke: Default::default(),
                }))
            }
            return shapes;
        }
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
                Text::default()
                    .with_text(text.as_str())
                    .with_color(color)
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
                callback: std::sync::Arc::new(move |render_ctx| {
                    if let Some(painter) = render_ctx.downcast_ref::<egui_glow::Painter>() {
                        let mut brush_mut = glyph_brush.lock().unwrap();
                        brush_mut.draw_queued(&painter.gl(),
                                              (gutter_rect.max.x - gutter_rect.min.x) as u32, (gutter_rect.max.y - gutter_rect.min.y) as u32)
                            .expect("Draw queued");
                    } else {
                        eprintln!("Can't do custom painting because we are not using a glow context");
                    }
                }),
            });
        });
    }
}