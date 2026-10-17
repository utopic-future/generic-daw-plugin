//! Built-in commands, the default keymap and key-event conversion.
//!
//! Every built-in command is a plain `daw::Message`. Plugins will register more
//! commands into the same `Registry` and can replace these by reusing a name.

use crate::daw::Message;
use generic_daw_script::{Chord, Handler, Keymap, Registry};
use iced::keyboard;
use std::sync::Arc;

/// A raw key press, forwarded from the subscription into `Daw::update`.
/// Boxed inside `Message::Key` so the 64-byte `Message` size assertion keeps holding.
#[derive(Clone, Debug)]
pub struct KeyEvent {
	pub key: keyboard::Key,
	pub physical_key: keyboard::key::Physical,
	pub modifiers: keyboard::Modifiers,
	pub repeat: bool,
}

pub fn builtin() -> Registry<Message> {
	let mut registry = Registry::default();

	for (name, title, message) in [
		("file:new", "New project", Message::NewFile),
		("file:open", "Open project", Message::OpenFileDialog),
		("file:open-last", "Open last project", Message::OpenLastFile),
		("file:save", "Save project", Message::SaveFile),
		("file:save-as", "Save project as", Message::SaveAsFileDialog),
		("file:render", "Render to WAV", Message::RenderFileDialog),
		("devices:reconnect", "Reconnect devices", Message::RescanDevices),
		("plugins:rescan", "Rescan plugins", Message::RescanPlugins),
		("plugins:picker", "Toggle plugin picker", Message::TogglePluginPicker),
		("settings:toggle", "Toggle settings", Message::ToggleConfigView),
		("transport:toggle-playback", "Play / pause", Message::TogglePlayback),
		("transport:stop", "Stop", Message::Stop),
		("transport:toggle-metronome", "Toggle metronome", Message::ToggleMetronome),
		("transport:tap-bpm", "Tap tempo", Message::TappedBpm),
		("view:toggle-fullscreen", "Toggle fullscreen", Message::ToggleFullscreen),
		("view:toggle-seconds", "Toggle seconds / beats display", Message::ToggleShowSeconds),
		("view:toggle-autoscroll", "Toggle autoscroll", Message::ToggleAutoscroll),
		("view:cycle-forwards", "Next tab", Message::CycleForwards),
		("view:cycle-backwards", "Previous tab", Message::CycleBackwards),
		("view:move-pane-up", "Move pane up", Message::MovePaneUp),
		("view:move-pane-down", "Move pane down", Message::MovePaneDown),
		("view:close-pane", "Close pane", Message::ClosePane),
		("grid:narrow", "Narrow grid", Message::NarrowedGrid),
		("grid:widen", "Widen grid", Message::WidenedGrid),
		("grid:toggle-triplets", "Toggle triplet grid", Message::ToggleGridTriplets),
		("grid:toggle", "Toggle grid", Message::ToggleGrid),
		("palette:toggle", "Command palette", Message::TogglePalette),
	] {
		registry.add(name, title, Handler::Native(message));
	}

	registry
}

/// Only the palette binding lives here for now. The existing hardcoded keybinds
/// (`Daw::keybinds` and the per-view ones) keep working as a fallback, and move
/// into this table one by one.
pub fn default_keymap() -> Keymap {
	let mut keymap = Keymap::default();
	keymap.bind(
		None,
		"ctrl+shift+p".parse().expect("valid chord"),
		"palette:toggle",
	);
	keymap
}

pub fn chord_from_event(
	key: &keyboard::Key,
	physical_key: keyboard::key::Physical,
	modifiers: keyboard::Modifiers,
) -> Option<Chord> {
	use keyboard::key::Named;

	let name: Arc<str> = match key.as_ref() {
		keyboard::Key::Named(named) => {
			let name: &str = match named {
				Named::Space => "space",
				Named::Tab => "tab",
				Named::Escape => "escape",
				Named::Enter => "enter",
				Named::Backspace => "backspace",
				Named::Delete => "delete",
				Named::ArrowUp => "up",
				Named::ArrowDown => "down",
				Named::ArrowLeft => "left",
				Named::ArrowRight => "right",
				Named::F11 => "f11",
				_ => return None,
			};
			name.into()
		}
		_ => key
			.to_latin(physical_key)?
			.to_lowercase()
			.collect::<String>()
			.into(),
	};

	Some(Chord {
		command: modifiers.command(),
		shift: modifiers.shift(),
		alt: modifiers.alt(),
		key: name,
	})
}
