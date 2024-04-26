use eframe::egui::{Color32, Pos2, Rect, Rounding, Shape, TextBuffer};
use eframe::epaint;
use eframe::epaint::RectShape;
use crate::text_editor::TextEditor;

pub trait Selection {
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