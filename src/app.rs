use crate::config::Config;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};

pub enum Outcome {
    Committed(String),
    Cancelled,
}

pub struct App {
    families: Vec<String>,
    filtered: Vec<usize>,
    query: String,
    list_state: ListState,
    matcher: Matcher,
    pending_write: Option<Instant>,
    last_written: Option<String>,
}

const DEBOUNCE: Duration = Duration::from_millis(120);

impl App {
    pub fn new(families: Vec<String>) -> Self {
        let filtered: Vec<usize> = (0..families.len()).collect();
        let mut list_state = ListState::default();
        if !filtered.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            families,
            filtered,
            query: String::new(),
            list_state,
            matcher: Matcher::new(nucleo_matcher::Config::DEFAULT),
            pending_write: None,
            last_written: None,
        }
    }

    fn refilter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.families.len()).collect();
        } else {
            let pattern = Pattern::parse(&self.query, CaseMatching::Ignore, Normalization::Smart);
            let mut scored: Vec<(usize, u32)> = self
                .families
                .iter()
                .enumerate()
                .filter_map(|(i, name)| {
                    let mut buf = Vec::new();
                    let haystack = Utf32Str::new(name, &mut buf);
                    pattern
                        .score(haystack, &mut self.matcher)
                        .map(|score| (i, score))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }

        self.list_state
            .select(if self.filtered.is_empty() { None } else { Some(0) });
    }

    fn selected_family(&self) -> Option<&str> {
        let idx = self.list_state.selected()?;
        let family_idx = *self.filtered.get(idx)?;
        self.families.get(family_idx).map(|s| s.as_str())
    }

    fn move_selection(&mut self, delta: i32) {
        if self.filtered.is_empty() {
            return;
        }
        let len = self.filtered.len() as i32;
        let current = self.list_state.selected().unwrap_or(0) as i32;
        let next = ((current + delta).rem_euclid(len)) as usize;
        self.list_state.select(Some(next));
        self.pending_write = Some(Instant::now() + DEBOUNCE);
    }

    fn maybe_flush_preview(&mut self, config: &mut Config) -> Result<()> {
        let Some(due) = self.pending_write else {
            return Ok(());
        };
        if Instant::now() < due {
            return Ok(());
        }
        self.pending_write = None;

        let Some(family) = self.selected_family() else {
            return Ok(());
        };
        if self.last_written.as_deref() == Some(family) {
            return Ok(());
        }
        let family = family.to_string();
        config.apply_family(&family)?;
        config.write()?;
        self.last_written = Some(family);
        Ok(())
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal, config: &mut Config) -> Result<Outcome> {
        // Preview the initially-selected candidate immediately.
        self.pending_write = Some(Instant::now());

        loop {
            self.maybe_flush_preview(config)?;
            terminal.draw(|f| self.draw(f))?;

            if !event::poll(Duration::from_millis(30))? {
                continue;
            }

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Esc => return Ok(Outcome::Cancelled),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(Outcome::Cancelled)
                }
                KeyCode::Enter => {
                    self.maybe_flush_preview(config)?;
                    if let Some(family) = self.selected_family() {
                        return Ok(Outcome::Committed(family.to_string()));
                    }
                    return Ok(Outcome::Cancelled);
                }
                KeyCode::Down => self.move_selection(1),
                KeyCode::Up => self.move_selection(-1),
                KeyCode::Backspace => {
                    self.query.pop();
                    self.refilter();
                    self.pending_write = Some(Instant::now() + DEBOUNCE);
                }
                KeyCode::Char(c) => {
                    self.query.push(c);
                    self.refilter();
                    self.pending_write = Some(Instant::now() + DEBOUNCE);
                }
                _ => {}
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);

        let input = Paragraph::new(self.query.as_str())
            .block(Block::default().borders(Borders::ALL).title("Filter"));
        frame.render_widget(input, chunks[0]);

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .map(|&i| ListItem::new(self.families[i].as_str()))
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Fonts (↑/↓ preview live · Enter commit · Esc cancel)"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, chunks[1], &mut self.list_state);
    }
}
