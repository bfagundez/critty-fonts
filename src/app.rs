use crate::config::Config;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};

const BG: Color = Color::Rgb(10, 4, 24);
const PANEL_BG: Color = Color::Rgb(16, 8, 36);
const NEON_CYAN: Color = Color::Rgb(0, 255, 249);
const NEON_MAGENTA: Color = Color::Rgb(255, 43, 214);
const NEON_YELLOW: Color = Color::Rgb(247, 255, 0);
const NEON_GREEN: Color = Color::Rgb(57, 255, 20);
const DIM: Color = Color::Rgb(120, 110, 150);
const FG: Color = Color::Rgb(220, 220, 240);

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
        frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        let title = Paragraph::new(Line::from(vec![
            Span::styled("⟨⟨ ", Style::default().fg(NEON_MAGENTA)),
            Span::styled(
                "CRITTY FONTS",
                Style::default()
                    .fg(NEON_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ⟩⟩", Style::default().fg(NEON_MAGENTA)),
            Span::raw("  "),
            Span::styled(
                self.selected_family().unwrap_or("—").to_string(),
                Style::default()
                    .fg(NEON_YELLOW)
                    .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            ),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().bg(PANEL_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(NEON_MAGENTA)),
        );
        frame.render_widget(title, chunks[0]);

        let input = Paragraph::new(Line::from(vec![
            Span::styled("❯ ", Style::default().fg(NEON_GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(self.query.as_str(), Style::default().fg(NEON_CYAN)),
            Span::styled("▌", Style::default().fg(NEON_CYAN).add_modifier(Modifier::SLOW_BLINK)),
        ]))
        .style(Style::default().bg(PANEL_BG))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(NEON_YELLOW))
                .title(Span::styled(
                    " filter ",
                    Style::default().fg(NEON_YELLOW).add_modifier(Modifier::BOLD),
                )),
        );
        frame.render_widget(input, chunks[1]);

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .map(|&i| ListItem::new(Span::styled(self.families[i].as_str(), Style::default().fg(FG))))
            .collect();
        let count_title = format!(" {} font{} ", self.filtered.len(), if self.filtered.len() == 1 { "" } else { "s" });
        let list = List::new(items)
            .style(Style::default().bg(BG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(NEON_CYAN))
                    .title(Span::styled(count_title, Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD))),
            )
            .highlight_style(
                Style::default()
                    .bg(NEON_MAGENTA)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(list, chunks[2], &mut self.list_state);

        let footer = Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(NEON_YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" preview live   ", Style::default().fg(DIM)),
            Span::styled("Enter", Style::default().fg(NEON_GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" commit   ", Style::default().fg(DIM)),
            Span::styled("Esc", Style::default().fg(NEON_MAGENTA).add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", Style::default().fg(DIM)),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(footer, chunks[3]);
    }
}
