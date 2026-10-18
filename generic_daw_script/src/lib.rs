//! Command registry, key chords, keymap and fuzzy search for Generic DAW.
//!
//! This crate knows nothing about the GUI or about any scripting language.
//! `M` is the host's message type; a native command is just a stored `M`.
//! A Lua handler variant is added in the next patch.

use std::{collections::HashMap, fmt, str::FromStr, sync::Arc};

// ───────────────────────────── chords ─────────────────────────────

/// A key combination such as `ctrl+shift+p`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
	/// Ctrl on Windows/Linux, Cmd on macOS (mirrors `iced::keyboard::Modifiers::command`).
	pub command: bool,
	pub shift: bool,
	pub alt: bool,
	/// Lowercase key name: one character, or a name like `space`, `tab`, `escape`, `f11`.
	pub key: Arc<str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChordError {
	Empty,
	MissingKey,
	UnknownModifier(String),
}

impl fmt::Display for ChordError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Empty => f.write_str("empty key chord"),
			Self::MissingKey => f.write_str("key chord has modifiers but no key"),
			Self::UnknownModifier(m) => write!(f, "unknown modifier `{m}`"),
		}
	}
}

impl std::error::Error for ChordError {}

impl FromStr for Chord {
	type Err = ChordError;

	/// Parses `"ctrl+shift+p"`, `"cmd+,"`, `"alt+up"`. The last `+`-separated part is the key.
	/// (The `+` key itself is spelled `plus`.)
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.trim();
		if s.is_empty() {
			return Err(ChordError::Empty);
		}

		let parts = s.split('+').map(str::trim).collect::<Vec<_>>();
		let (key, modifiers) = parts.split_last().ok_or(ChordError::Empty)?;
		if key.is_empty() {
			return Err(ChordError::MissingKey);
		}

		let mut chord = Self {
			command: false,
			shift: false,
			alt: false,
			key: key.to_lowercase().into(),
		};

		for modifier in modifiers {
			match modifier.to_lowercase().as_str() {
				"ctrl" | "control" | "cmd" | "command" | "mod" => chord.command = true,
				"shift" => chord.shift = true,
				"alt" | "option" => chord.alt = true,
				other => return Err(ChordError::UnknownModifier(other.to_owned())),
			}
		}

		Ok(chord)
	}
}

impl fmt::Display for Chord {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.command {
			f.write_str("Ctrl+")?;
		}
		if self.shift {
			f.write_str("Shift+")?;
		}
		if self.alt {
			f.write_str("Alt+")?;
		}

		let mut chars = self.key.chars();
		match chars.next() {
			Some(first) => write!(f, "{}{}", first.to_uppercase(), chars.as_str()),
			None => Ok(()),
		}
	}
}

// ───────────────────────────── keymap ─────────────────────────────

#[derive(Clone, Debug)]
struct Binding {
	context: Option<Arc<str>>,
	chord: Chord,
	command: Arc<str>,
}

/// Maps `(context, chord)` to a command name.
///
/// `None` is the global context. Lookups try the given contexts in order
/// (highest priority first), then the global context. Within one context the
/// most recently added binding wins, so plugins can override built-ins.
#[derive(Clone, Debug, Default)]
pub struct Keymap {
	bindings: Vec<Binding>,
}

impl Keymap {
	pub fn bind(&mut self, context: Option<&str>, chord: Chord, command: impl Into<Arc<str>>) {
		self.bindings.push(Binding {
			context: context.map(Arc::from),
			chord,
			command: command.into(),
		});
	}

	/// Removes every binding of `chord` in `context`.
	pub fn unbind(&mut self, context: Option<&str>, chord: &Chord) {
		self.bindings
			.retain(|b| !(b.context.as_deref() == context && b.chord == *chord));
	}

	pub fn resolve(&self, contexts: &[&str], chord: &Chord) -> Option<&Arc<str>> {
		contexts
			.iter()
			.map(|&c| Some(c))
			.chain(std::iter::once(None))
			.find_map(|context| {
				self.bindings
					.iter()
					.rev()
					.find(|b| b.context.as_deref() == context && b.chord == *chord)
					.map(|b| &b.command)
			})
	}

	/// The most recently bound chord for `command`, for display in menus and the palette.
	pub fn chord_for(&self, command: &str) -> Option<&Chord> {
		self.bindings
			.iter()
			.rev()
			.find(|b| &*b.command == command)
			.map(|b| &b.chord)
	}
}

// ──────────────────────────── commands ────────────────────────────

#[derive(Clone, Debug)]
pub enum Handler<M> {
	/// Runs by feeding `M` back into the host's `update`.
	Native(M),
}

#[derive(Clone, Debug)]
pub struct Command<M> {
	/// Stable id, e.g. `file:save`.
	pub name: Arc<str>,
	/// Human-readable title shown in the palette.
	pub title: Arc<str>,
	pub handler: Handler<M>,
}

#[derive(Clone, Debug)]
pub struct Registry<M> {
	commands: Vec<Command<M>>,
	index: HashMap<Arc<str>, usize>,
}

impl<M> Default for Registry<M> {
	fn default() -> Self {
		Self {
			commands: Vec::new(),
			index: HashMap::new(),
		}
	}
}

impl<M> Registry<M> {
	/// Adds a command, or replaces the existing one with the same name in place
	/// (this is how plugins will override built-ins).
	pub fn add(
		&mut self,
		name: impl Into<Arc<str>>,
		title: impl Into<Arc<str>>,
		handler: Handler<M>,
	) {
		let name = name.into();
		let command = Command {
			name: name.clone(),
			title: title.into(),
			handler,
		};

		let existing = self.index.get(&name).copied();
		match existing {
			Some(i) => self.commands[i] = command,
			None => {
				self.index.insert(name, self.commands.len());
				self.commands.push(command);
			}
		}
	}

	pub fn get(&self, name: &str) -> Option<&Command<M>> {
		self.index.get(name).map(|&i| &self.commands[i])
	}

	pub fn iter(&self) -> impl Iterator<Item = &Command<M>> {
		self.commands.iter()
	}

	pub fn len(&self) -> usize {
		self.commands.len()
	}

	pub fn is_empty(&self) -> bool {
		self.commands.is_empty()
	}

	/// Commands matching `query` (against title or name), best first.
	/// An empty query returns everything in registration order.
	pub fn search(&self, query: &str) -> Vec<&Command<M>> {
		let mut hits = self
			.commands
			.iter()
			.filter_map(|c| {
				let score = fuzzy_score(query, &c.title)
					.into_iter()
					.chain(fuzzy_score(query, &c.name))
					.max()?;
				Some((score, c))
			})
			.collect::<Vec<_>>();

		// stable sort: ties keep registration order
		hits.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
		hits.into_iter().map(|(_, c)| c).collect()
	}
}

// ───────────────────────────── fuzzy ─────────────────────────────

/// Subsequence match. `None` if `query` isn't a subsequence of `text`.
/// Consecutive matches and matches at word starts score higher; shorter text wins ties.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
	let query = query
		.chars()
		.filter(|c| !c.is_whitespace())
		.flat_map(char::to_lowercase)
		.collect::<Vec<_>>();
	if query.is_empty() {
		return Some(0);
	}

	let mut matched = 0;
	let mut score = 0;
	let mut prev_matched = false;
	let mut prev_char = None::<char>;
	let mut len = 0;

	for c in text.chars().flat_map(char::to_lowercase) {
		len += 1;
		if matched < query.len() && c == query[matched] {
			score += 1;
			if prev_matched {
				score += 3;
			}
			if prev_char.is_none_or(|p| !p.is_alphanumeric()) {
				score += 5;
			}
			matched += 1;
			prev_matched = true;
		} else {
			prev_matched = false;
		}
		prev_char = Some(c);
	}

	(matched == query.len()).then_some(score * 10 - len)
}

// ───────────────────────────── tests ─────────────────────────────

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_and_prints_chords() {
		let chord = "Ctrl+Shift+P".parse::<Chord>().unwrap();
		assert!(chord.command && chord.shift && !chord.alt);
		assert_eq!(&*chord.key, "p");
		assert_eq!(chord.to_string(), "Ctrl+Shift+P");
		assert_eq!("alt+up".parse::<Chord>().unwrap().to_string(), "Alt+Up");
		assert_eq!("cmd+,".parse::<Chord>().unwrap().to_string(), "Ctrl+,");
	}

	#[test]
	fn rejects_bad_chords() {
		assert_eq!("".parse::<Chord>(), Err(ChordError::Empty));
		assert_eq!("ctrl+".parse::<Chord>(), Err(ChordError::MissingKey));
		assert!(matches!(
			"hyper+x".parse::<Chord>(),
			Err(ChordError::UnknownModifier(_))
		));
	}

	#[test]
	fn keymap_later_wins_and_context_beats_global() {
		let p = "ctrl+p".parse::<Chord>().unwrap();
		let mut keymap = Keymap::default();

		keymap.bind(None, p.clone(), "a");
		keymap.bind(None, p.clone(), "b");
		assert_eq!(keymap.resolve(&[], &p).map(|c| &**c), Some("b"));

		keymap.bind(Some("palette"), p.clone(), "c");
		assert_eq!(keymap.resolve(&["palette"], &p).map(|c| &**c), Some("c"));
		assert_eq!(keymap.resolve(&["other"], &p).map(|c| &**c), Some("b"));

		keymap.unbind(Some("palette"), &p);
		assert_eq!(keymap.resolve(&["palette"], &p).map(|c| &**c), Some("b"));
		assert_eq!(keymap.chord_for("b"), Some(&p));
	}

	#[test]
	fn registry_adds_replaces_and_searches() {
		let mut registry = Registry::<u32>::default();
		registry.add("file:save", "Save project", Handler::Native(1));
		registry.add("file:save-as", "Save project as", Handler::Native(2));
		registry.add("transport:stop", "Stop", Handler::Native(3));
		assert_eq!(registry.len(), 3);

		// replace keeps position and count
		registry.add("file:save", "Save project", Handler::Native(10));
		assert_eq!(registry.len(), 3);
		let Handler::Native(n) = registry.get("file:save").unwrap().handler.clone();
		assert_eq!(n, 10);

		// shorter title ranks first among equal matches
		let hits = registry.search("sav");
		assert_eq!(hits.len(), 2);
		assert_eq!(&*hits[0].name, "file:save");

		// only "Stop" contains s..t..p as a subsequence
		let hits = registry.search("stp");
		assert_eq!(hits.len(), 1);
		assert_eq!(&*hits[0].name, "transport:stop");

		// empty query: everything, registration order
		let names = registry
			.search("")
			.iter()
			.map(|c| &*c.name)
			.collect::<Vec<_>>();
		assert_eq!(names, ["file:save", "file:save-as", "transport:stop"]);
	}

	#[test]
	fn fuzzy_rejects_non_subsequences() {
		assert_eq!(fuzzy_score("xyz", "Save project"), None);
		assert!(fuzzy_score("sp", "Save project") > fuzzy_score("sp", "Resample"));
	}
}
