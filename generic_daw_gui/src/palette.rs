//! Command palette overlay. Same shape as `plugin_picker`: its own `Message`,
//! an `update` returning `Action<Instruction, Message>`, and a `view` the parent
//! stacks on top of the main layout.

use crate::{
	action::Action,
	daw::{Instruction, Message as DawMessage},
	stylefns::{button_with_radius, container_with_radius, weaker_bordered_box},
};
use generic_daw_script::{Keymap, Registry};
use iced::{
	Element, Fill, keyboard,
	widget::{button, column, container, row, scrollable, text, text_input},
};
use std::sync::{Arc, LazyLock};

const MAX_RESULTS: usize = 50;

// FIXME(api): written against iced 0.13/0.14 (`text_input::Id` + `text_input::focus`).
// Your fork is 0.15-dev, so if this doesn't compile, adjust these two lines
// (and the `.id(...)` call in `view`) to the fork's focus API.
pub static INPUT_ID: LazyLock<text_input::Id> =
	LazyLock::new(|| text_input::Id::new("command-palette"));

#[derive(Clone, Debug)]
pub enum Message {
	QueryChanged(String),
	Up,
	Down,
	Submit,
	Pick(usize),
}

#[derive(Debug)]
pub struct Palette {
	query: String,
	results: Vec<Arc<str>>,
	selected: usize,
}

impl Palette {
	pub fn new(registry: &Registry<DawMessage>) -> Self {
		let mut this = Self {
			query: String::new(),
			results: Vec::new(),
			selected: 0,
		};
		this.refresh(registry);
		this
	}

	fn refresh(&mut self, registry: &Registry<DawMessage>) {
		self.results = registry
			.search(&self.query)
			.into_iter()
			.take(MAX_RESULTS)
			.map(|command| command.name.clone())
			.collect();
		self.selected = 0;
	}

	pub fn update(
		&mut self,
		message: Message,
		registry: &Registry<DawMessage>,
	) -> Action<Instruction, Message> {
		match message {
			Message::QueryChanged(query) => {
				self.query = query;
				self.refresh(registry);
				Action::none()
			}
			Message::Up => {
				self.selected = self.selected.saturating_sub(1);
				Action::none()
			}
			Message::Down => {
				if self.selected + 1 < self.results.len() {
					self.selected += 1;
				}
				Action::none()
			}
			Message::Submit => self.run(self.selected),
			Message::Pick(i) => self.run(i),
		}
	}

	fn run(&self, i: usize) -> Action<Instruction, Message> {
		self.results.get(i).map_or_else(Action::none, |name| {
			Action::instruction(Instruction::Message(DawMessage::RunFromPalette(
				name.clone(),
			)))
		})
	}

	pub fn view<'a>(
		&'a self,
		registry: &'a Registry<DawMessage>,
		keymap: &'a Keymap,
	) -> Element<'a, Message> {
		let input = text_input("Type a command…", &self.query)
			.id(INPUT_ID.clone())
			.on_input(Message::QueryChanged)
			.on_submit(Message::Submit)
			.padding(8);

		let rows = column(
			self.results
				.iter()
				.enumerate()
				.filter_map(|(i, name)| {
					let command = registry.get(name)?;
					let hint = keymap
						.chord_for(name)
						.map(ToString::to_string)
						.unwrap_or_default();

					Some(
						button(
							row![text(&*command.title).width(Fill), text(hint).size(13)]
								.spacing(10),
						)
						.width(Fill)
						.style(button_with_radius(
							if i == self.selected {
								button::primary
							} else {
								button::subtle
							},
							5,
						))
						.on_press(Message::Pick(i))
						.into(),
					)
				}),
		)
		.spacing(2);

		container(
			column![input, scrollable(rows).height(Fill.max(320))]
				.spacing(8)
				.padding(10),
		)
		.width(480)
		.style(container_with_radius(weaker_bordered_box, 10))
		.into()
	}

	/// Keys handled while the palette is open (Enter is handled by the text input's `on_submit`).
	pub fn keybinds(
		key: &keyboard::Key,
		_modifiers: keyboard::Modifiers,
		_repeat: bool,
	) -> Option<DawMessage> {
		match key.as_ref() {
			keyboard::Key::Named(keyboard::key::Named::Escape) => Some(DawMessage::TogglePalette),
			keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
				Some(DawMessage::Palette(Message::Up))
			}
			keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
				Some(DawMessage::Palette(Message::Down))
			}
			_ => None,
		}
	}
}
