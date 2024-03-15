use serde::{Deserialize, Serialize};
use std::{io, io::stdout};

use color_eyre::config::HookBuilder;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, style::palette::tailwind, widgets::*};

const TODO_HEADER_BG: Color = tailwind::BLUE.c950;
const NORMAL_ROW_COLOR: Color = tailwind::SLATE.c950;
const SELECTED_STYLE_FG: Color = tailwind::BLUE.c300;
const TEXT_COLOR: Color = tailwind::SLATE.c200;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    theme_name: Option<String>,

    #[arg(short, long, help = "Print current theme name")]
    print_current_theme: bool,
}

#[derive(Serialize, Deserialize)]
struct YmlPrimary {
    background: String,
    foreground: String,
}

impl Default for YmlPrimary {
    fn default() -> Self {
        Self {
            background: "#000000".into(),
            foreground: "#000000".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct YmlCursor {
    text: String,
    cursor: String,
}

impl Default for YmlCursor {
    fn default() -> Self {
        Self {
            text: "#000000".into(),
            cursor: "#000000".into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct YmlNormal {
    black: String,
    red: String,
    green: String,
    yellow: String,
    blue: String,
    magenta: String,
    cyan: String,
    white: String,
}

impl Default for YmlNormal {
    fn default() -> Self {
        Self {
            black: "#000000".into(),
            red: "#000000".into(),
            green: "#000000".into(),
            yellow: "#000000".into(),
            blue: "#000000".into(),
            magenta: "#000000".into(),
            cyan: "#000000".into(),
            white: "#000000".into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct YmlBright {
    black: String,
    red: String,
    green: String,
    yellow: String,
    blue: String,
    magenta: String,
    cyan: String,
    white: String,
}

impl Default for YmlBright {
    fn default() -> Self {
        Self {
            black: "#000000".into(),
            red: "#000000".into(),
            green: "#000000".into(),
            yellow: "#000000".into(),
            blue: "#000000".into(),
            magenta: "#000000".into(),
            cyan: "#000000".into(),
            white: "#000000".into(),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct YmlColors {
    name: Option<String>,
    author: Option<String>,
    primary: YmlPrimary,
    cursor: Option<YmlCursor>,
    normal: YmlNormal,
    bright: YmlBright,
}

#[derive(Default, Serialize, Deserialize)]
struct YmlColor {
    colors: YmlColors,
}

fn find_alacritty_config_file() -> String {
    let file = String::new();

    let mut home = match env::var("HOME") {
        Ok(v) => PathBuf::from(v),
        Err(e) => {
            eprintln!("Failed to get HOME env var!: {e}");
            exit(1);
        }
    };

    home.push("alacritty.yml");

    if home.exists() {
        return home.to_str().unwrap().to_string();
    }

    let mut xdg_config_home: PathBuf = match env::var("XDG_CONFIG_HOME") {
        Ok(v) => PathBuf::from(v),
        Err(e) => {
            eprintln!("Failed to get XDG_CONFIG_HOME env var!: {e}");
            exit(1);
        }
    };

    xdg_config_home.push("alacritty");
    xdg_config_home.push("alacritty.yml");

    if xdg_config_home.exists() {
        return xdg_config_home.to_str().unwrap().to_string();
    }

    file
}

fn get_themes_dir() -> PathBuf {
    let mut dir: PathBuf = match env::var("XDG_CONFIG_HOME") {
        Ok(v) => PathBuf::from(v),
        Err(e) => {
            eprintln!("Failed to get XDG_CONFIG_HOME env var!: {e}");
            exit(1);
        }
    };

    dir.push("alacritty");
    dir.push("themes");

    dir
}

// backup the main config file before doing any changes
// first check if a backup already exists if not backup the file
fn backup_cfg_file(file: &String) {
    let mut backup_file = PathBuf::from(file);

    // TODO: Rewrite this code
    // Change 'alacritty.yml' -> 'alacritty-backup.yml'
    let new_name = format!(
        "{}-backup.{}",
        backup_file.as_path().file_stem().unwrap().to_str().unwrap(),
        backup_file.as_path().extension().unwrap().to_str().unwrap()
    );
    backup_file.set_file_name(new_name);

    if !backup_file.exists() {
        // backup config file
        match fs::copy(file, &backup_file) {
            Ok(_) => {
                println!("backup: {} -> {}", file, backup_file.to_str().unwrap());
            }
            Err(e) => {
                eprintln!("Failed to backup alacritty config file: {e}");
            }
        }
    }
}

fn apply_theme(file_path: &String, theme_path: &str) {
    let alacritty_cfg_contents = fs::read_to_string(file_path).expect("File not found");
    let theme_file_contents = fs::read_to_string(theme_path).expect("File not found");

    let mut color: serde_yaml::Value = serde_yaml::from_str(&alacritty_cfg_contents).unwrap();
    let new_theme_color: serde_yaml::Value = serde_yaml::from_str(&theme_file_contents).unwrap();

    color["colors"] = new_theme_color["colors"].clone();

    let new_cfg_file = serde_yaml::to_string(&color).unwrap();

    match fs::write(file_path, new_cfg_file) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to write to alacritty config file: {e}");
        }
    }
}

struct ListItem {
    name: String,
    path: String,
}

struct StatefulList {
    state: ListState,
    alacritty_cfg_file: String,
    items: Vec<ListItem>,
    last_selected: Option<usize>,
}

struct App {
    items: StatefulList,
}

fn get_themes() -> Vec<ListItem> {
    let alacritty_config_file_path: String = find_alacritty_config_file();
    let themes_dir: PathBuf = get_themes_dir();
    let mut themes_list: Vec<ListItem> = Vec::new();

    if !themes_dir.exists() {
        eprintln!("Themes dir not found!");
        exit(1);
    }

    if !Path::new(&alacritty_config_file_path).exists() {
        eprintln!("Config file not found!");
        exit(1);
    }

    backup_cfg_file(&alacritty_config_file_path);

    for entry in themes_dir.read_dir().unwrap() {
        match entry {
            Ok(file) => {
                let item = ListItem {
                    name: file
                        .path()
                        .as_path()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                    path: file.path().as_path().to_str().unwrap().to_string(),
                };
                themes_list.push(item);
            }
            Err(e) => {
                eprintln!("Failed to read_dir on file: {e}");
                exit(1);
            }
        }
    }

    themes_list
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.print_current_theme {
        let alacritty_cfg_path = find_alacritty_config_file();

        if !Path::new(&alacritty_cfg_path).exists() {
            eprintln!("alacritty cfg: '{}' not found!", alacritty_cfg_path);
            exit(1);
        }

        let alacritty_cfg_str = fs::read_to_string(alacritty_cfg_path).unwrap();
        let file: YmlColor = serde_yaml::from_str(&alacritty_cfg_str).unwrap();

        println!(
            "{}",
            file.colors.name.unwrap_or("ERROR: name not found".into())
        );

        exit(0);
    }

    if let Some(theme_name) = cli.theme_name {
        let mut themes_dir = get_themes_dir();

        if !theme_name.ends_with(".yml") {
            themes_dir.push(format!("{}.yml", theme_name));
        } else {
            themes_dir.push(theme_name.clone());
        }

        if !themes_dir.exists() {
            eprintln!("Theme '{}' not found", theme_name);
            exit(1);
        }

        let alacritty_cfg = find_alacritty_config_file();

        apply_theme(&alacritty_cfg, themes_dir.as_path().to_str().unwrap());

        exit(0);
    }

    init_error_hooks()?;
    let terminal = init_terminal()?;

    // create app and run it
    App::new().run(terminal)?;

    restore_terminal()?;

    Ok(())
}

fn init_error_hooks() -> color_eyre::Result<()> {
    let (panic, error) = HookBuilder::default().into_hooks();
    let panic = panic.into_panic_hook();
    let error = error.into_eyre_hook();
    color_eyre::eyre::set_hook(Box::new(move |e| {
        let _ = restore_terminal();
        error(e)
    }))?;
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        panic(info);
    }));
    Ok(())
}

fn init_terminal() -> color_eyre::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> color_eyre::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn render_title(area: Rect, buf: &mut Buffer) {
    Paragraph::new("Themes switcher")
        .bold()
        .centered()
        .render(area, buf);
}

fn render_footer(area: Rect, buf: &mut Buffer) {
    Paragraph::new("\nUse ↓↑ to move, a to apply theme, g/G to go top/bottom.")
        .centered()
        .render(area, buf);
}

impl StatefulList {
    fn with_items(items: Vec<ListItem>) -> StatefulList {
        StatefulList {
            state: ListState::default(),
            items,
            alacritty_cfg_file: find_alacritty_config_file(),
            last_selected: None,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => self.last_selected.unwrap_or(0),
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => self.last_selected.unwrap_or(0),
        };
        self.state.select(Some(i));
    }
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

fn hex_to_rgb(s: String) -> Color {
    // #fff is the min
    if s.is_empty() || s.len() < 3 {
        return Color::Rgb(0, 0, 0);
    }

    let mut hex: String = s;

    if hex.starts_with('#') {
        hex = hex.replace('#', "");
    } else if hex.starts_with("0x") {
        hex = hex.replace("0x", "");
    }

    if hex.len() < 6 {
        let c = hex.chars().next().unwrap();
        let max = 6 - hex.len();

        for _ in 0..max {
            hex.push(c);
        }
    }

    match decode_hex(&hex) {
        Ok(v) => Color::Rgb(v[0], v[1], v[2]),
        Err(_) => Color::Rgb(0, 0, 0),
    }
}

impl App {
    fn new() -> Self {
        Self {
            items: StatefulList::with_items(get_themes()),
        }
    }

    fn go_top(&mut self) {
        self.items.state.select(Some(0));
    }

    fn go_bottom(&mut self) {
        self.items.state.select(Some(self.items.items.len() - 1));
    }

    fn apply_theme(&self) {
        let theme_index = self.items.state.selected().unwrap();

        let theme = self.items.items.get(theme_index);

        if let Some(theme) = theme {
            apply_theme(&self.items.alacritty_cfg_file, &theme.path);
        }
    }
}

impl App {
    fn run(&mut self, mut terminal: Terminal<impl Backend>) -> io::Result<()> {
        loop {
            self.draw(&mut terminal)?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    use KeyCode::*;
                    match key.code {
                        Char('q') | Esc => return Ok(()),
                        Char('j') | Down => self.items.next(),
                        Char('k') | Up => self.items.previous(),
                        Char('g') => self.go_top(),
                        Char('G') => self.go_bottom(),
                        Char('a') => self.apply_theme(),
                        _ => {}
                    }
                }
            }
        }
    }

    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        terminal.draw(|f| f.render_widget(self, f.size()))?;
        Ok(())
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create a space for header, todo list and the footer.
        let vertical = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ]);
        let [header_area, rest_area, footer_area] = vertical.areas(area);

        // Create two chunks with equal vertical screen space. One for the list and the other for
        // the info block.
        let horizontal =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]);
        let [upper_item_list_area, lower_item_list_area] = horizontal.areas(rest_area);

        render_title(header_area, buf);
        self.render_todo(upper_item_list_area, buf);
        self.render_info(lower_item_list_area, buf);
        render_footer(footer_area, buf);
    }
}

impl App {
    fn render_todo(&mut self, area: Rect, buf: &mut Buffer) {
        // We create two blocks, one is for the header (outer) and the other is for list (inner).
        let outer_block = Block::default()
            .borders(Borders::NONE)
            .fg(TEXT_COLOR)
            .bg(TODO_HEADER_BG)
            .title("Themes List")
            .title_alignment(Alignment::Center);
        let inner_block = Block::default()
            .borders(Borders::NONE)
            .fg(TEXT_COLOR)
            .bg(NORMAL_ROW_COLOR);

        // We get the inner area from outer_block. We'll use this area later to render the table.
        let outer_area = area;
        let inner_area = outer_block.inner(outer_area);

        // We can render the header in outer_area.
        outer_block.render(outer_area, buf);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<Line> = self
            .items
            .items
            .iter()
            .map(|item| Line::styled(item.name.to_string(), TEXT_COLOR))
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let items = List::new(items)
            .block(inner_block)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::REVERSED)
                    .fg(SELECTED_STYLE_FG),
            )
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        // We can now render the item list
        // (look careful we are using StatefulWidget's render.)
        // ratatui::widgets::StatefulWidget::render as stateful_render
        StatefulWidget::render(items, inner_area, buf, &mut self.items.state);
    }

    fn render_info(&self, area: Rect, buf: &mut Buffer) {
        let theme_index = self.items.state.selected().unwrap_or(0);
        let theme_path = self.items.items.get(theme_index).unwrap();
        // get theme file and parse it
        let theme_file_contents =
            fs::read_to_string(&theme_path.path).expect("Failed to read theme"); // TODO: Remove .expect later

        let theme_colors: YmlColor =
            serde_yaml::from_str(&theme_file_contents).unwrap_or(YmlColor::default());
        let colors = theme_colors.colors;
        // TODO: Make fg visable no mater the bg color

        let info: Vec<Line> = vec![
            Line::from(vec![
                Span::raw("name:"),
                Span::styled(
                    colors.name.unwrap_or("Empty".to_string()),
                    Style::new().bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("author:"),
                Span::styled(
                    colors.author.unwrap_or("Empty".to_string()),
                    Style::new().bold(),
                ),
            ]),
            // ---
            Line::from("primary:"),
            Line::from(vec![
                Span::raw("background:"),
                Span::styled(
                    colors.primary.background.clone(),
                    Style::new()
                        .bg(hex_to_rgb(colors.primary.background))
                        .bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("foreground:"),
                Span::styled(
                    colors.primary.foreground.clone(),
                    Style::new()
                        .bg(hex_to_rgb(colors.primary.foreground))
                        .bold(),
                ),
            ]),
            // ---
            Line::from("cursor:"),
            Line::from(vec![
                Span::raw("text:"),
                Span::styled(
                    match colors.cursor.clone() {
                        Some(c) => c.text,
                        None => "Empty".to_string(),
                    },
                    Style::new()
                        .bg(hex_to_rgb(colors.cursor.clone().unwrap_or_default().text))
                        .bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("cursor:"),
                Span::styled(
                    match colors.cursor.clone() {
                        Some(c) => c.cursor,
                        None => "Empty".to_string(),
                    },
                    Style::new()
                        .bg(hex_to_rgb(colors.cursor.unwrap_or_default().cursor))
                        .bold(),
                ),
            ]),
            // ---
            Line::from("normal:"),
            Line::from(vec![
                Span::raw("black:"),
                Span::styled(
                    colors.normal.black.clone(),
                    Style::new().bg(hex_to_rgb(colors.normal.black)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("red:"),
                Span::styled(
                    colors.normal.red.clone(),
                    Style::new().bg(hex_to_rgb(colors.normal.red)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("green:"),
                Span::styled(
                    colors.normal.green.clone(),
                    Style::new().bg(hex_to_rgb(colors.normal.green)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("yellow:"),
                Span::styled(
                    colors.normal.yellow.clone(),
                    Style::new().bg(hex_to_rgb(colors.normal.yellow)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("blue:"),
                Span::styled(
                    colors.normal.blue.clone(),
                    Style::new().bg(hex_to_rgb(colors.normal.blue)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("magenta:"),
                Span::styled(
                    colors.normal.magenta.clone(),
                    Style::new().bg(hex_to_rgb(colors.normal.magenta)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("cyan:"),
                Span::styled(
                    colors.normal.cyan.clone(),
                    Style::new().bg(hex_to_rgb(colors.normal.cyan)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("white:"),
                Span::styled(
                    colors.normal.white.clone(),
                    Style::new().bg(hex_to_rgb(colors.normal.white)).bold(),
                ),
            ]),
            // ---
            Line::from("bright:"),
            Line::from(vec![
                Span::raw("black:"),
                Span::styled(
                    colors.bright.black.clone(),
                    Style::new().bg(hex_to_rgb(colors.bright.black)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("red:"),
                Span::styled(
                    colors.bright.red.clone(),
                    Style::new().bg(hex_to_rgb(colors.bright.red)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("green:"),
                Span::styled(
                    colors.bright.green.clone(),
                    Style::new().bg(hex_to_rgb(colors.bright.green)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("yellow:"),
                Span::styled(
                    colors.bright.yellow.clone(),
                    Style::new().bg(hex_to_rgb(colors.bright.yellow)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("blue:"),
                Span::styled(
                    colors.bright.blue.clone(),
                    Style::new().bg(hex_to_rgb(colors.bright.blue)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("magenta:"),
                Span::styled(
                    colors.bright.magenta.clone(),
                    Style::new().bg(hex_to_rgb(colors.bright.magenta)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("cyan:"),
                Span::styled(
                    colors.bright.cyan.clone(),
                    Style::new().bg(hex_to_rgb(colors.bright.cyan)).bold(),
                ),
            ]),
            Line::from(vec![
                Span::raw("white:"),
                Span::styled(
                    colors.bright.white.clone(),
                    Style::new().bg(hex_to_rgb(colors.bright.white)).bold(),
                ),
            ]),
            // ---
        ];

        // We show the list item's info under the list in this paragraph
        let outer_info_block = Block::default()
            .borders(Borders::NONE)
            .fg(TEXT_COLOR)
            .bg(TODO_HEADER_BG)
            .title("Info")
            .title_alignment(Alignment::Center);
        let inner_info_block = Block::default()
            .borders(Borders::NONE)
            .bg(NORMAL_ROW_COLOR)
            .padding(Padding::horizontal(1));

        // This is a similar process to what we did for list. outer_info_area will be used for
        // header inner_info_area will be used for the list info.
        let outer_info_area = area;
        let inner_info_area = outer_info_block.inner(outer_info_area);

        // We can render the header. Inner info will be rendered later
        outer_info_block.render(outer_info_area, buf);

        let info_paragraph = Paragraph::new(info)
            .block(inner_info_block)
            .fg(TEXT_COLOR)
            .wrap(Wrap { trim: false });

        info_paragraph.render(inner_info_area, buf);
    }
}
