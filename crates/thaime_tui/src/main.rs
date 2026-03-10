// SPDX-License-Identifier: MPL-2.0

//! THAIME TUI - Interactive test harness for exploring and tuning the engine.
//!
//! ## Modes
//!
//! - **Main** — Live candidate exploration with score decomposition and parameter tuning.
//! - **Lattice** — View all word lattice edges for the current input.
//! - **Inspector** — Trie explorer with optional "why not?" target word diagnosis.
//! - **Regression** — Run and view regression test results.
//!
//! ## Keybinds
//!
//! All action keybinds use modifier or function keys to avoid conflicts with
//! text input. See the status bar hints in each mode.

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation,
              ScrollbarState, Table, TableState, Wrap},
    DefaultTerminal, Frame,
};

use serde::{Deserialize, Serialize};

use thaime_engine::ranking::{
    rank_candidates, Candidate, LatticeEdge, RankingParams, DEFAULT_K, DEFAULT_LAMBDA,
    DEFAULT_MIN_FREQ,
};
use thaime_engine::trie::Dictionary;

const MAX_INPUT_LEN: usize = 50;
const EPSILON_PRESETS: &[f64] = &[1e-4, 1e-5, 1e-6, 1e-7];
const DEFAULT_TEST_FILE: &str = "tests/harness-regression.toml";
const MIN_TERMINAL_WIDTH: u16 = 60;
const MIN_TERMINAL_HEIGHT: u16 = 15;

// ---------------------------------------------------------------------------
// Regression test types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestFile {
    #[serde(default)]
    test: Vec<TestPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestPair {
    input: String,
    expected_thai: String,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Clone)]
struct TestResult {
    pair: TestPair,
    passed: bool,
    actual_thai: Option<String>,
    actual_rank: Option<usize>,
    actual_score: Option<f64>,
    expected_score: Option<f64>,
}

// ---------------------------------------------------------------------------
// Inspector types
// ---------------------------------------------------------------------------

/// A flattened trie match entry for display in inspector mode.
#[derive(Debug, Clone)]
struct InspectorEntry {
    position: usize,
    key: String,
    word_id: u32,
    thai: String,
    frequency: f64,
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Main,
    Lattice,
    Inspector,
    Regression,
}

/// Overlay prompt state for save confirmation.
#[derive(Debug, Clone)]
enum SaveState {
    /// No save in progress.
    Idle,
    /// Showing confirmation: "Save test: input → thai?"
    Confirming { input: String, thai: String },
    /// Entering a note before saving.
    EnteringNote {
        input: String,
        thai: String,
        note: String,
    },
}

struct App {
    mode: Mode,
    should_quit: bool,

    // Main mode: input + ranking
    input: String,
    dictionary: Dictionary,
    candidates: Vec<Candidate>,
    lattice_edges: Vec<LatticeEdge>,
    scoring_duration: Duration,

    // Tunable parameters
    params: RankingParams,
    epsilon_idx: usize,

    // Inspector mode
    inspector_input: String,
    inspector_entries: Vec<InspectorEntry>,
    inspector_target: Option<String>,
    inspector_entering_target: bool,
    inspector_target_input: String,

    // Regression tests
    test_file_path: PathBuf,
    test_results: Vec<TestResult>,
    show_failures_only: bool,

    // Save prompt overlay
    save_state: SaveState,

    // Help overlay
    show_help: bool,

    // Status message (briefly shown after actions)
    status_message: Option<String>,

    // UI state — TableState drives both scrolling and row selection
    candidate_table_state: TableState,
    lattice_table_state: TableState,
    regression_table_state: TableState,
}

impl App {
    fn new(dictionary: Dictionary) -> Self {
        Self {
            mode: Mode::Main,
            should_quit: false,
            input: String::new(),
            dictionary,
            candidates: Vec::new(),
            lattice_edges: Vec::new(),
            scoring_duration: Duration::ZERO,
            params: RankingParams::default(),
            epsilon_idx: 0,
            inspector_input: String::new(),
            inspector_entries: Vec::new(),
            inspector_target: None,
            inspector_entering_target: false,
            inspector_target_input: String::new(),
            test_file_path: PathBuf::from(DEFAULT_TEST_FILE),
            test_results: Vec::new(),
            show_failures_only: false,
            save_state: SaveState::Idle,
            show_help: false,
            status_message: None,
            candidate_table_state: TableState::default(),
            lattice_table_state: TableState::default(),
            regression_table_state: TableState::default(),
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.ui(frame))?;

            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                // Clear status message on any keypress
                self.status_message = None;
                self.handle_key(key);
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Key handling
    // -----------------------------------------------------------------------

    fn handle_key(&mut self, key: KeyEvent) {
        // Help overlay: any key dismisses it
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Save prompt overlay takes priority
        if !matches!(self.save_state, SaveState::Idle) {
            self.handle_save_key(key);
            return;
        }

        // Global: F1 or ? opens help
        if key.code == KeyCode::F(1)
            || (key.code == KeyCode::Char('?') && !matches!(self.mode, Mode::Inspector))
        {
            self.show_help = true;
            return;
        }

        match self.mode {
            Mode::Main => self.handle_main_key(key),
            Mode::Lattice => self.handle_lattice_key(key),
            Mode::Inspector => self.handle_inspector_key(key),
            Mode::Regression => self.handle_regression_key(key),
        }
    }

    fn handle_main_key(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => self.should_quit = true,

            // Mode switches
            (KeyCode::F(2), _) => {
                self.mode = Mode::Lattice;
                self.lattice_table_state = TableState::default();
            }
            (KeyCode::F(3), _) => {
                self.mode = Mode::Inspector;
                // Copy current input to inspector if inspector is empty
                if self.inspector_input.is_empty() && !self.input.is_empty() {
                    self.inspector_input.clone_from(&self.input);
                    self.refresh_inspector();
                }
            }
            (KeyCode::F(5), _) => {
                self.run_regression_tests();
                self.mode = Mode::Regression;
                self.regression_table_state = TableState::default();
            }

            // Save test (Ctrl+S)
            (KeyCode::Char('s'), m) if m.contains(KeyModifiers::CONTROL) => {
                if !self.input.is_empty() && !self.candidates.is_empty() {
                    let thai = self.candidates[0].thai.clone();
                    self.save_state = SaveState::Confirming {
                        input: self.input.clone(),
                        thai,
                    };
                }
            }

            // Parameter tuning: λ
            (KeyCode::Up, m) if m.contains(KeyModifiers::CONTROL) => {
                self.params.lambda = (self.params.lambda + 0.1).min(5.0);
                self.refresh();
            }
            (KeyCode::Down, m) if m.contains(KeyModifiers::CONTROL) => {
                self.params.lambda = ((self.params.lambda - 0.1) * 10.0).round() / 10.0;
                if self.params.lambda < 0.0 {
                    self.params.lambda = 0.0;
                }
                self.refresh();
            }

            // Parameter tuning: k
            (KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => {
                self.params.k = (self.params.k + 5).min(50);
                self.refresh();
            }
            (KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => {
                self.params.k = self.params.k.saturating_sub(5).max(5);
                self.refresh();
            }

            // Parameter tuning: ε
            (KeyCode::Char('e'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.epsilon_idx = (self.epsilon_idx + 1) % EPSILON_PRESETS.len();
                self.params.min_freq = EPSILON_PRESETS[self.epsilon_idx];
                self.refresh();
            }

            // Clear input
            (KeyCode::Char('u'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.input.clear();
                self.refresh();
            }

            // Backspace
            (KeyCode::Backspace, _) => {
                if self.input.pop().is_some() {
                    self.refresh();
                }
            }

            // Text input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                if c.is_ascii_alphabetic() && self.input.len() < MAX_INPUT_LEN {
                    self.input.push(c.to_ascii_lowercase());
                    self.refresh();
                }
            }

            _ => {}
        }
    }

    fn handle_lattice_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::F(2) => self.mode = Mode::Main,
            KeyCode::Up => self.lattice_table_state.scroll_up_by(1),
            KeyCode::Down => self.lattice_table_state.scroll_down_by(1),
            KeyCode::Home => self.lattice_table_state.select_first(),
            KeyCode::End => self.lattice_table_state.select_last(),
            _ => {}
        }
    }

    fn handle_inspector_key(&mut self, key: KeyEvent) {
        if self.inspector_entering_target {
            // Target input sub-mode
            match key.code {
                KeyCode::Esc => {
                    self.inspector_entering_target = false;
                    self.inspector_target_input.clear();
                }
                KeyCode::Enter => {
                    let input = self.inspector_target_input.trim().to_string();
                    if input.is_empty() {
                        // Clear target
                        self.inspector_target = None;
                    } else {
                        self.inspector_target = Some(input);
                    }
                    self.inspector_entering_target = false;
                    self.inspector_target_input.clear();
                }
                KeyCode::Backspace => {
                    self.inspector_target_input.pop();
                }
                KeyCode::Char(c) => {
                    self.inspector_target_input.push(c);
                }
                _ => {}
            }
            return;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Esc | KeyCode::F(3), _) => self.mode = Mode::Main,

            // Set/clear target (Ctrl+T)
            (KeyCode::Char('t'), m) if m.contains(KeyModifiers::CONTROL) => {
                if self.inspector_target.is_some() {
                    // Clear target
                    self.inspector_target = None;
                } else {
                    self.inspector_entering_target = true;
                    self.inspector_target_input.clear();
                }
            }

            // Clear input
            (KeyCode::Char('u'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.inspector_input.clear();
                self.inspector_entries.clear();
            }

            // Backspace
            (KeyCode::Backspace, _) => {
                if self.inspector_input.pop().is_some() {
                    self.refresh_inspector();
                }
            }

            // Text input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                if c.is_ascii_alphabetic() && self.inspector_input.len() < MAX_INPUT_LEN {
                    self.inspector_input.push(c.to_ascii_lowercase());
                    self.refresh_inspector();
                }
            }

            _ => {}
        }
    }

    fn handle_regression_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Main,
            KeyCode::Char('f') => self.show_failures_only = !self.show_failures_only,
            KeyCode::Up => self.regression_table_state.scroll_up_by(1),
            KeyCode::Down => self.regression_table_state.scroll_down_by(1),
            KeyCode::Home => self.regression_table_state.select_first(),
            KeyCode::End => self.regression_table_state.select_last(),
            KeyCode::F(5) => self.run_regression_tests(),
            _ => {}
        }
    }

    fn handle_save_key(&mut self, key: KeyEvent) {
        match &self.save_state.clone() {
            SaveState::Idle => {}
            SaveState::Confirming { input, thai } => match key.code {
                KeyCode::Enter => {
                    self.save_test_pair(input.clone(), thai.clone(), None);
                    self.save_state = SaveState::Idle;
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.save_state = SaveState::EnteringNote {
                        input: input.clone(),
                        thai: thai.clone(),
                        note: String::new(),
                    };
                }
                KeyCode::Esc => {
                    self.save_state = SaveState::Idle;
                }
                _ => {}
            },
            SaveState::EnteringNote { input, thai, note } => match key.code {
                KeyCode::Enter => {
                    let note = if note.is_empty() {
                        None
                    } else {
                        Some(note.clone())
                    };
                    self.save_test_pair(input.clone(), thai.clone(), note);
                    self.save_state = SaveState::Idle;
                }
                KeyCode::Esc => {
                    self.save_state = SaveState::Idle;
                }
                KeyCode::Backspace => {
                    let mut note = note.clone();
                    note.pop();
                    self.save_state = SaveState::EnteringNote {
                        input: input.clone(),
                        thai: thai.clone(),
                        note,
                    };
                }
                KeyCode::Char(c) => {
                    let mut note = note.clone();
                    note.push(c);
                    self.save_state = SaveState::EnteringNote {
                        input: input.clone(),
                        thai: thai.clone(),
                        note,
                    };
                }
                _ => {}
            },
        }
    }

    // -----------------------------------------------------------------------
    // Engine interaction
    // -----------------------------------------------------------------------

    fn refresh(&mut self) {
        if self.input.is_empty() {
            self.candidates.clear();
            self.lattice_edges.clear();
            self.scoring_duration = Duration::ZERO;
        } else {
            let start = Instant::now();
            let result = rank_candidates(&self.input, &self.dictionary, &self.params);
            self.scoring_duration = start.elapsed();
            self.candidates = result.candidates;
            self.lattice_edges = result.lattice_edges;
        }
    }

    fn refresh_inspector(&mut self) {
        self.inspector_entries.clear();
        if self.inspector_input.is_empty() {
            return;
        }

        let input = &self.inspector_input;
        for pos in 0..input.len() {
            let suffix = &input[pos..];
            for pm in self.dictionary.prefix_search(suffix) {
                let key = suffix[..pm.prefix_len].to_string();
                for entry in &pm.entries {
                    self.inspector_entries.push(InspectorEntry {
                        position: pos,
                        key: key.clone(),
                        word_id: entry.word_id,
                        thai: entry.thai.clone(),
                        frequency: entry.frequency,
                    });
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Regression test management
    // -----------------------------------------------------------------------

    fn load_test_pairs(&mut self) -> Vec<TestPair> {
        let content = match fs::read_to_string(&self.test_file_path) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Vec::new(),
            Err(e) => {
                self.status_message = Some(format!(
                    "Failed to read {}: {}",
                    self.test_file_path.display(),
                    e
                ));
                return Vec::new();
            }
        };
        match toml::from_str::<TestFile>(&content) {
            Ok(file) => file.test,
            Err(e) => {
                self.status_message = Some(format!("Malformed TOML: {}", e));
                Vec::new()
            }
        }
    }

    fn run_regression_tests(&mut self) {
        let pairs = self.load_test_pairs();
        self.test_results = pairs
            .into_iter()
            .map(|pair| {
                let result = rank_candidates(&pair.input, &self.dictionary, &self.params);
                let candidates = &result.candidates;

                if candidates.is_empty() {
                    return TestResult {
                        pair,
                        passed: false,
                        actual_thai: None,
                        actual_rank: None,
                        actual_score: None,
                        expected_score: None,
                    };
                }

                let actual_thai = candidates[0].thai.clone();
                let passed = actual_thai == pair.expected_thai;

                // Find where the expected word actually ranked
                let expected_pos = candidates
                    .iter()
                    .position(|c| c.thai == pair.expected_thai);
                let expected_score = expected_pos.map(|i| candidates[i].score);

                TestResult {
                    passed,
                    actual_thai: Some(actual_thai),
                    actual_rank: expected_pos.map(|i| i + 1),
                    actual_score: Some(candidates[0].score),
                    expected_score,
                    pair,
                }
            })
            .collect();
    }

    fn save_test_pair(&mut self, input: String, thai: String, note: Option<String>) {
        let pair = TestPair {
            input,
            expected_thai: thai,
            note,
        };

        // Read existing content or start fresh
        let mut content = fs::read_to_string(&self.test_file_path).unwrap_or_default();

        // Append new entry
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!(
            "\n[[test]]\ninput = \"{}\"\nexpected_thai = \"{}\"",
            pair.input, pair.expected_thai
        ));
        if let Some(ref note) = pair.note {
            content.push_str(&format!("\nnote = \"{}\"", note));
        }
        content.push('\n');

        // Ensure parent directory exists
        if let Some(parent) = self.test_file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        match fs::write(&self.test_file_path, &content) {
            Ok(()) => {
                let count = self.load_test_pairs().len();
                self.status_message = Some(format!("Test saved (total: {} tests)", count));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to save: {}", e));
            }
        }
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    fn ui(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Minimum terminal size check
        if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
            let msg = Paragraph::new(format!(
                "Terminal too small ({}x{})\nMinimum: {}x{}",
                area.width, area.height, MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT,
            ))
            .style(Style::default().fg(Color::Red));
            frame.render_widget(msg, area);
            return;
        }

        match self.mode {
            Mode::Main => self.render_main(frame),
            Mode::Lattice => self.render_lattice(frame),
            Mode::Inspector => self.render_inspector(frame),
            Mode::Regression => self.render_regression(frame),
        }

        // Save prompt overlay (rendered on top of current mode)
        if !matches!(self.save_state, SaveState::Idle) {
            self.render_save_overlay(frame);
        }

        // Help overlay (rendered on top of everything)
        if self.show_help {
            self.render_help_overlay(frame);
        }
    }

    fn render_main(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Input
                Constraint::Min(5),    // Candidates
                Constraint::Length(3), // Status + hints
            ])
            .split(area);

        // --- Input box ---
        let cursor_indicator = if self.input.is_empty() { "│" } else { "" };
        let input_text = format!("> {}{}", self.input, cursor_indicator);
        let input_widget = Paragraph::new(input_text)
            .block(Block::default().borders(Borders::ALL).title(" Input "));
        frame.render_widget(input_widget, chunks[0]);

        #[allow(clippy::cast_possible_truncation)]
        frame.set_cursor_position((
            chunks[0].x + 3 + self.input.len() as u16,
            chunks[0].y + 1,
        ));

        // --- Candidate table ---
        let param_changed = self.params.lambda != DEFAULT_LAMBDA
            || self.params.min_freq != DEFAULT_MIN_FREQ
            || self.params.k != DEFAULT_K;

        let title = if param_changed {
            format!(
                " Candidates (k={}, λ={:.1}, ε={:.0e}) ",
                self.params.k, self.params.lambda, self.params.min_freq
            )
        } else {
            " Candidates ".to_string()
        };

        let header = Row::new([
            Cell::from("#"),
            Cell::from("Thai"),
            Cell::from("Score"),
            Cell::from("Freq Cost"),
            Cell::from("Seg Pen"),
            Cell::from("Words"),
        ])
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        );

        let rows: Vec<Row> = self
            .candidates
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let thai_display = if c.word_count() > 1 {
                    c.words
                        .iter()
                        .map(|w| w.thai.as_str())
                        .collect::<Vec<_>>()
                        .join("+")
                } else {
                    c.thai.clone()
                };
                Row::new([
                    Cell::from(format!("{}", i + 1)),
                    Cell::from(thai_display),
                    Cell::from(format!("{:.2}", c.score)),
                    Cell::from(format!("{:.2}", c.freq_cost)),
                    Cell::from(format!("{:.2}", c.seg_penalty)),
                    Cell::from(format!("{}", c.word_count())),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(3),
            Constraint::Min(16),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(6),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(title))
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(table, chunks[1], &mut self.candidate_table_state);

        // --- Status bar ---
        let status_line = if let Some(ref msg) = self.status_message {
            Line::from(vec![Span::styled(
                format!(" {}", msg),
                Style::default().fg(Color::Green),
            )])
        } else if self.input.is_empty() {
            Line::from(vec![Span::styled(
                " Ready",
                Style::default().fg(Color::DarkGray),
            )])
        } else {
            Line::from(vec![Span::raw(format!(
                " Edges: {} │ Scored in: {:.0?} │ λ={:.1} ε={:.0e} k={}",
                self.lattice_edges.len(),
                self.scoring_duration,
                self.params.lambda,
                self.params.min_freq,
                self.params.k,
            ))])
        };

        let hints = Line::from(vec![Span::styled(
            " [F1] Help  [F2] Lattice  [F3] Inspector  [F5] Tests  [Ctrl+S] Save  [Esc] Quit",
            Style::default().fg(Color::DarkGray),
        )]);

        let status = Paragraph::new(vec![status_line, hints]);
        frame.render_widget(status, chunks[2]);
    }

    fn render_lattice(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(area);

        // --- Input display ---
        let input_display = if self.input.is_empty() {
            "(empty)".to_string()
        } else {
            self.input
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i > 0 {
                        format!(" · {}", c)
                    } else {
                        c.to_string()
                    }
                })
                .collect()
        };

        let input_widget = Paragraph::new(format!("  {}", input_display)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Lattice for: \"{}\" ", self.input)),
        );
        frame.render_widget(input_widget, chunks[0]);

        // --- Lattice edges table ---
        if self.lattice_edges.is_empty() {
            let msg = Paragraph::new("  (no lattice edges — type something in main mode)")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title(" Edges (0) "));
            frame.render_widget(msg, chunks[1]);
        } else {
            let mut sorted_edges: Vec<&LatticeEdge> = self.lattice_edges.iter().collect();
            sorted_edges.sort_by(|a, b| {
                a.start
                    .cmp(&b.start)
                    .then((b.end - b.start).cmp(&(a.end - a.start)))
            });

            let header = Row::new([
                Cell::from("Span"),
                Cell::from("Thai"),
                Cell::from("Cost"),
                Cell::from("Freq"),
                Cell::from("Word ID"),
            ])
            .style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            );

            let rows: Vec<Row> = sorted_edges
                .iter()
                .map(|e| {
                    Row::new([
                        Cell::from(format!("[{}..{})", e.start, e.end)),
                        Cell::from(e.thai.clone()),
                        Cell::from(format!("{:.2}", e.cost)),
                        Cell::from(format!("{:.2e}", e.frequency)),
                        Cell::from(format!("{}", e.word_id)),
                    ])
                })
                .collect();

            let edge_count = rows.len();
            let widths = [
                Constraint::Length(10),
                Constraint::Min(14),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Length(8),
            ];

            let table = Table::new(rows, widths)
                .header(header)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Edges ({}) ", edge_count)),
                )
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            frame.render_stateful_widget(table, chunks[1], &mut self.lattice_table_state);

            if edge_count > chunks[1].height.saturating_sub(3) as usize {
                let scroll_pos = self.lattice_table_state.selected().unwrap_or(0);
                let mut scrollbar_state =
                    ScrollbarState::new(edge_count).position(scroll_pos);
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
                frame.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
            }
        }

        let hints = Paragraph::new(Line::from(vec![Span::styled(
            " [Esc] Back to main  [↑↓] Scroll",
            Style::default().fg(Color::DarkGray),
        )]));
        frame.render_widget(hints, chunks[2]);
    }

    fn render_inspector(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: input, target display, trie matches, diagnosis (if target set), hints
        let has_target = self.inspector_target.is_some();
        let constraints = if has_target {
            vec![
                Constraint::Length(3),  // Input
                Constraint::Length(3),  // Target
                Constraint::Min(5),    // Trie matches
                Constraint::Length(8),  // Diagnosis
                Constraint::Length(2),  // Hints
            ]
        } else {
            vec![
                Constraint::Length(3),  // Input
                Constraint::Length(3),  // Target
                Constraint::Min(5),    // Trie matches
                Constraint::Length(2),  // Hints
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // --- Input box ---
        let cursor_indicator = if self.inspector_input.is_empty() { "│" } else { "" };
        let input_text = format!("> {}{}", self.inspector_input, cursor_indicator);
        let input_widget = Paragraph::new(input_text)
            .block(Block::default().borders(Borders::ALL).title(" Inspector Input "));
        frame.render_widget(input_widget, chunks[0]);

        if !self.inspector_entering_target {
            #[allow(clippy::cast_possible_truncation)]
            frame.set_cursor_position((
                chunks[0].x + 3 + self.inspector_input.len() as u16,
                chunks[0].y + 1,
            ));
        }

        // --- Target display ---
        let target_text = if self.inspector_entering_target {
            format!(
                "  Enter target (Thai text or word ID): {}│",
                self.inspector_target_input
            )
        } else if let Some(ref target) = self.inspector_target {
            format!("  Target: {}  (Ctrl+T to clear)", target)
        } else {
            "  (no target — Ctrl+T to set)".to_string()
        };

        let target_style = if self.inspector_entering_target {
            Style::default().fg(Color::Yellow)
        } else if self.inspector_target.is_some() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let target_widget = Paragraph::new(target_text).style(target_style).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Target "),
        );
        frame.render_widget(target_widget, chunks[1]);

        if self.inspector_entering_target {
            #[allow(clippy::cast_possible_truncation)]
            frame.set_cursor_position((
                chunks[1].x + 40 + self.inspector_target_input.len() as u16,
                chunks[1].y + 1,
            ));
        }

        // --- Trie matches table ---
        let match_chunk = chunks[2];

        let header = Row::new([
            Cell::from("Pos"),
            Cell::from("Key"),
            Cell::from("Thai"),
            Cell::from("Word ID"),
            Cell::from("Freq"),
            Cell::from(""),
        ])
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        );

        let target = self.inspector_target.as_deref();

        let rows: Vec<Row> = self
            .inspector_entries
            .iter()
            .map(|e| {
                let is_target = target.is_some_and(|t| e.thai == t);
                let marker = if is_target { "◀ TARGET" } else { "" };
                let style = if is_target {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                Row::new([
                    Cell::from(format!("{}", e.position)),
                    Cell::from(e.key.clone()),
                    Cell::from(e.thai.clone()),
                    Cell::from(format!("{}", e.word_id)),
                    Cell::from(format!("{:.2e}", e.frequency)),
                    Cell::from(marker),
                ])
                .style(style)
            })
            .collect();

        let match_count = rows.len();
        let widths = [
            Constraint::Length(4),
            Constraint::Length(12),
            Constraint::Min(12),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths).header(header).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Trie Prefix Matches ({}) ", match_count)),
        );
        frame.render_widget(table, match_chunk);

        // --- Diagnosis (if target set) ---
        if has_target {
            let diagnosis_chunk = chunks[3];
            let diagnosis_lines = self.build_diagnosis();
            let diagnosis = Paragraph::new(diagnosis_lines)
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Target Diagnosis "),
                );
            frame.render_widget(diagnosis, diagnosis_chunk);
        }

        // --- Hints ---
        let hint_chunk = if has_target { chunks[4] } else { chunks[3] };
        let hints = Paragraph::new(Line::from(vec![Span::styled(
            " [Esc] Back to main  [Ctrl+T] Set/clear target",
            Style::default().fg(Color::DarkGray),
        )]));
        frame.render_widget(hints, hint_chunk);
    }

    fn build_diagnosis(&self) -> Vec<Line<'static>> {
        let Some(ref target) = self.inspector_target else {
            return vec![Line::from("  No target set.")];
        };

        let mut lines = Vec::new();

        // Check if target is found in any trie match
        let target_entries: Vec<&InspectorEntry> = self
            .inspector_entries
            .iter()
            .filter(|e| e.thai == *target)
            .collect();

        if target_entries.is_empty() {
            lines.push(Line::from(Span::styled(
                "  ✗ Target not found in trie matches for this input",
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(
                "    The target word may not be in the dictionary, or its romanization",
            ));
            lines.push(Line::from(
                "    does not match any prefix of the current input.",
            ));
            return lines;
        }

        // Target found in trie
        for entry in &target_entries {
            lines.push(Line::from(Span::styled(
                format!(
                    "  ✓ Found via key \"{}\" at pos {} (word_id={})",
                    entry.key, entry.position, entry.word_id
                ),
                Style::default().fg(Color::Green),
            )));
        }

        // Run ranking to see where it lands
        if !self.inspector_input.is_empty() {
            let result = rank_candidates(&self.inspector_input, &self.dictionary, &self.params);
            let candidates = &result.candidates;

            if let Some(rank) = candidates.iter().position(|c| c.thai == *target) {
                lines.push(Line::from(format!(
                    "  Candidate rank: #{} (score: {:.2})",
                    rank + 1,
                    candidates[rank].score
                )));
                if rank > 0 {
                    lines.push(Line::from(format!(
                        "  #1 is: {} (score: {:.2}, Δ={:+.2})",
                        candidates[0].thai,
                        candidates[0].score,
                        candidates[rank].score - candidates[0].score
                    )));
                }
            } else if !candidates.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!(
                        "  ✗ Target not in top {} candidates",
                        candidates.len()
                    ),
                    Style::default().fg(Color::Red),
                )));
                lines.push(Line::from(format!(
                    "    #1 is: {} (score: {:.2})",
                    candidates[0].thai, candidates[0].score
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "  ✗ No complete candidates for this input",
                    Style::default().fg(Color::Red),
                )));
            }
        }

        lines
    }

    fn render_regression(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),   // Results
                Constraint::Length(3), // Summary
                Constraint::Length(2), // Hints
            ])
            .split(area);

        let total = self.test_results.len();
        let passed = self.test_results.iter().filter(|r| r.passed).count();
        let failed = total - passed;

        // --- Results table ---
        let header = Row::new([
            Cell::from(""),
            Cell::from("Input"),
            Cell::from("Expected"),
            Cell::from("Actual (#1)"),
            Cell::from("Rank"),
            Cell::from("Score"),
            Cell::from("Note"),
        ])
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        );

        let filtered: Vec<&TestResult> = self
            .test_results
            .iter()
            .filter(|r| !self.show_failures_only || !r.passed)
            .collect();

        let rows: Vec<Row> = filtered
            .iter()
            .map(|r| {
                let icon = if r.passed { "✓" } else { "✗" };
                let style = if r.passed {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };
                let actual = r
                    .actual_thai
                    .as_deref()
                    .unwrap_or("(none)");
                let rank = r
                    .actual_rank
                    .map(|n| format!("#{}", n))
                    .unwrap_or_else(|| "N/A".to_string());
                let score = match (r.actual_score, r.expected_score) {
                    (Some(actual), Some(expected)) if !r.passed => {
                        format!("{:.2} (Δ{:+.2})", actual, expected - actual)
                    }
                    (Some(actual), _) => format!("{:.2}", actual),
                    _ => String::new(),
                };
                let note = r.pair.note.as_deref().unwrap_or("");

                Row::new([
                    Cell::from(icon),
                    Cell::from(r.pair.input.clone()),
                    Cell::from(r.pair.expected_thai.clone()),
                    Cell::from(actual.to_string()),
                    Cell::from(rank),
                    Cell::from(score),
                    Cell::from(note.to_string()),
                ])
                .style(style)
            })
            .collect();

        let display_count = rows.len();
        let widths = [
            Constraint::Length(2),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Min(10),
        ];

        let filter_label = if self.show_failures_only {
            " (failures only)"
        } else {
            ""
        };

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default().borders(Borders::ALL).title(format!(
                    " Regression Tests ({}{}) ",
                    display_count, filter_label
                )),
            )
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(table, chunks[0], &mut self.regression_table_state);

        if display_count > chunks[0].height.saturating_sub(3) as usize {
            let scroll_pos = self.regression_table_state.selected().unwrap_or(0);
            let mut scrollbar_state =
                ScrollbarState::new(display_count).position(scroll_pos);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            frame.render_stateful_widget(scrollbar, chunks[0], &mut scrollbar_state);
        }

        // --- Summary ---
        let pass_rate = if total > 0 {
            format!("{:.1}%", passed as f64 / total as f64 * 100.0)
        } else {
            "N/A".to_string()
        };

        let summary = Paragraph::new(vec![
            Line::from(format!(
                "  Passed: {}/{}  ({})  │  Failed: {}  │  λ={:.1}  ε={:.0e}  k={}",
                passed,
                total,
                pass_rate,
                failed,
                self.params.lambda,
                self.params.min_freq,
                self.params.k,
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Summary "),
        );
        frame.render_widget(summary, chunks[1]);

        // --- Hints ---
        let hints = Paragraph::new(Line::from(vec![Span::styled(
            " [Esc] Back to main  [f] Toggle failures only  [F5] Re-run  [↑↓] Scroll",
            Style::default().fg(Color::DarkGray),
        )]));
        frame.render_widget(hints, chunks[2]);
    }

    fn render_save_overlay(&self, frame: &mut Frame) {
        let area = frame.area();
        // Center a popup
        let popup = centered_rect(60, 7, area);

        frame.render_widget(Clear, popup);

        let lines = match &self.save_state {
            SaveState::Confirming { input, thai } => {
                vec![
                    Line::from(""),
                    Line::from(format!("  Save test: \"{}\" → {}?", input, thai)),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  [Enter] Save  [Ctrl+N] Add note  [Esc] Cancel",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            }
            SaveState::EnteringNote { input, thai, note } => {
                vec![
                    Line::from(format!("  Test: \"{}\" → {}", input, thai)),
                    Line::from(""),
                    Line::from(format!("  Note: {}│", note)),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  [Enter] Save  [Esc] Cancel",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            }
            SaveState::Idle => vec![],
        };

        let popup_widget = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Save Regression Test ")
                .style(Style::default().fg(Color::White)),
        );
        frame.render_widget(popup_widget, popup);
    }

    fn render_help_overlay(&self, frame: &mut Frame) {
        let area = frame.area();
        let popup = centered_rect(70, 24, area);
        frame.render_widget(Clear, popup);

        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let key_style = Style::default().fg(Color::Yellow);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("  Main Mode", header_style)),
            Line::from(vec![
                Span::styled("    F1 / ?      ", key_style),
                Span::raw("Help (this overlay)"),
            ]),
            Line::from(vec![
                Span::styled("    F2          ", key_style),
                Span::raw("Lattice mode"),
            ]),
            Line::from(vec![
                Span::styled("    F3          ", key_style),
                Span::raw("Inspector mode"),
            ]),
            Line::from(vec![
                Span::styled("    F5          ", key_style),
                Span::raw("Run regression tests"),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+S      ", key_style),
                Span::raw("Save current input as regression test"),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+↑/↓    ", key_style),
                Span::raw("Adjust λ ±0.1"),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+←/→    ", key_style),
                Span::raw("Adjust k ±5"),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+E      ", key_style),
                Span::raw("Cycle ε presets"),
            ]),
            Line::from(vec![
                Span::styled("    Ctrl+U      ", key_style),
                Span::raw("Clear input"),
            ]),
            Line::from(vec![
                Span::styled("    Esc         ", key_style),
                Span::raw("Quit"),
            ]),
            Line::from(""),
            Line::from(Span::styled("  Lattice Mode", header_style)),
            Line::from(vec![
                Span::styled("    ↑/↓         ", key_style),
                Span::raw("Scroll"),
            ]),
            Line::from(vec![
                Span::styled("    Esc         ", key_style),
                Span::raw("Back to main"),
            ]),
            Line::from(""),
            Line::from(Span::styled("  Inspector Mode", header_style)),
            Line::from(vec![
                Span::styled("    Ctrl+T      ", key_style),
                Span::raw("Set/clear target word"),
            ]),
            Line::from(vec![
                Span::styled("    Esc         ", key_style),
                Span::raw("Back to main"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Press any key to close",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let help_widget = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help — Keybinds ")
                .style(Style::default().fg(Color::White)),
        );
        frame.render_widget(help_widget, popup);
    }
}

/// Create a centered rectangle for popup overlays.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    Rect::new(
        area.x + x,
        area.y + y,
        popup_width.min(area.width),
        height.min(area.height),
    )
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn print_usage() {
    println!(
        "THAIME TUI v{} — Interactive test harness for the Thai IME engine",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("Usage: thaime_tui [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --test-file <PATH>  Regression test file (default: {})", DEFAULT_TEST_FILE);
    println!("  --help              Show this help message");
    println!();
    println!("In-app help: press F1 or ? for keybind reference.");
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut test_file: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            "--test-file" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --test-file requires an argument");
                    process::exit(1);
                }
                test_file = Some(PathBuf::from(&args[i]));
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                print_usage();
                process::exit(1);
            }
        }
        i += 1;
    }

    let dict = Dictionary::from_embedded();
    let mut app = App::new(dict);
    if let Some(path) = test_file {
        app.test_file_path = path;
    }

    // Panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();

    result
}
