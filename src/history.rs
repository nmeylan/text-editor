use eframe::egui::{TextBuffer, Ui};
use crate::text_editor::{Pos, TextEditor};

#[derive(Clone, Debug)]
pub enum SingleAction {
    AddChar(AddCharAction),
    RemoveChar(RemoveCharAction),
    RemoveLine(usize),
    NewLine(Pos<usize>),
}

#[derive(Clone, Debug)]
pub enum BulkAction {
    AddText(TextAction),
    RemoveText(TextAction),
}

#[derive(Default, Clone, Debug)]
pub struct TextAction {
    pub start_index: usize,
    pub end_index: usize,
    pub lines: Vec<String>,
}

#[derive(Default, Clone, Debug)]
pub struct AddCharAction {
    pub start_pos: Pos<usize>,
    pub char: String,
}

#[derive(Default, Clone, Debug)]
pub struct RemoveCharAction {
    pub start_pos: Pos<usize>,
    pub char: char,
}

#[derive(Default, Clone, Debug)]
pub struct DefaultAction {
    pub start_pos: Pos<usize>,
    pub line: String,
}

#[derive(Default, Clone, Debug)]
pub struct UnsavedState {
    pub last_activity_at: f64,
    pub cursor_index: Pos<usize>,
    pub cursor_pos: Pos<f32>,
    pub actions: Vec<SingleAction>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub created_at: f64,
    pub cursor_index: Pos<usize>,
    pub cursor_pos: Pos<f32>,
    pub bulk_action: BulkAction,
}

pub trait HasUnsavedState {
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
            self.init_unsaved_state(ui.input().time);
        }
        let unsaved_state = self.unsaved_stated.as_mut().unwrap();
        unsaved_state.actions.push(action);
        if ui.input().time - unsaved_state.last_activity_at >= InactivityPeriod {
            self.feed_history(ui);
        } else {
            unsaved_state.last_activity_at = ui.input().time;
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