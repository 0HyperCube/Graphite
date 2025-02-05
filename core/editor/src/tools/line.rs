use crate::events::{Event, Response};
use crate::events::{Key, MouseKeys, ViewportPosition};
use crate::tools::{Fsm, Tool};
use crate::Document;
use document_core::layers::style;
use document_core::Operation;

use super::DocumentToolData;

#[derive(Default)]
pub struct Line {
	fsm_state: LineToolFsmState,
	data: LineToolData,
}

impl Tool for Line {
	fn handle_input(&mut self, event: &Event, document: &Document, tool_data: &DocumentToolData) -> (Vec<Response>, Vec<Operation>) {
		let mut responses = Vec::new();
		let mut operations = Vec::new();
		self.fsm_state = self.fsm_state.transition(event, document, tool_data, &mut self.data, &mut responses, &mut operations);

		(responses, operations)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineToolFsmState {
	Ready,
	LmbDown,
}

impl Default for LineToolFsmState {
	fn default() -> Self {
		LineToolFsmState::Ready
	}
}
#[derive(Clone, Debug, Default)]
struct LineToolData {
	drag_start: ViewportPosition,
}

impl Fsm for LineToolFsmState {
	type ToolData = LineToolData;

	fn transition(self, event: &Event, document: &Document, tool_data: &DocumentToolData, data: &mut Self::ToolData, responses: &mut Vec<Response>, operations: &mut Vec<Operation>) -> Self {
		match (self, event) {
			(LineToolFsmState::Ready, Event::MouseDown(mouse_state)) if mouse_state.mouse_keys.contains(MouseKeys::LEFT) => {
				data.drag_start = mouse_state.position;
				LineToolFsmState::LmbDown
			}
			(LineToolFsmState::Ready, Event::KeyDown(Key::KeyZ)) => {
				if let Some(id) = document.root.list_layers().last() {
					operations.push(Operation::DeleteLayer { path: vec![*id] })
				}
				LineToolFsmState::Ready
			}
			// TODO - Check for left mouse button
			(LineToolFsmState::LmbDown, Event::MouseUp(mouse_state)) => {
				let distance = data.drag_start.distance(&mouse_state.position);
				log::info!("draw Line with distance: {:.2}", distance);
				let start = data.drag_start;
				let end = mouse_state.position;
				operations.push(Operation::AddLine {
					path: vec![],
					insert_index: -1,
					x0: start.x as f64,
					y0: start.y as f64,
					x1: end.x as f64,
					y1: end.y as f64,
					style: style::PathStyle::new(Some(style::Stroke::new(tool_data.primary_color, 5.)), None),
				});

				LineToolFsmState::Ready
			}

			_ => self,
		}
	}
}
