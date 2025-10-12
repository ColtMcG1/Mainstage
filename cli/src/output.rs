/// cli/src/output.rs
/// Output utilities for CLI
/// author: Colton McGraw
/// date: October 10th, 2025
/// description: This module provides utilities for rendering progress bars,
/// spinners, and other output formatting in the CLI.
/// license: TBD

/// ====================================================================
/// Progress bar

/// Defines the frames for a progress bar
/// # Example
/// ```
/// let chars = ProgressCharacterSet::boxes();
/// let pb = Progress::new(10).with_progress_characters(chars);
/// ```
pub struct ProgressCharacterSet {
    pub left_container: Option<char>,
    pub complete: char,
    pub current: char,
    pub incomplete: char,
    pub right_container: Option<char>,
}

impl ProgressCharacterSet {
    pub fn new(left_container: Option<char>, complete: char, current: char, incomplete: char, right_container: Option<char>) -> Self {
        ProgressCharacterSet {
            left_container,
            complete,
            current,
            incomplete,
            right_container,
        }
    }
}

/// Predefined progress bar character sets
impl ProgressCharacterSet {
    pub fn boxes() -> Self {
        ProgressCharacterSet {
            left_container: Some('│'),
            complete: '█',
            current: '▒',
            incomplete: '░',
            right_container: Some('│'),
        }
    }
    pub fn arrow() -> Self {
        ProgressCharacterSet {
            left_container: Some('['),
            complete: '=',
            current: '>',
            incomplete: ' ',
            right_container: Some(']'),
        }
    }
    pub fn circles() -> Self {
        ProgressCharacterSet {
            left_container: Some('('),
            complete: '●',
            current: '◐',
            incomplete: '○',
            right_container: Some(')'),
        }
    }
    pub fn stars() -> Self {
        ProgressCharacterSet {
            left_container: Some('('),
            complete: '*',
            current: '+',
            incomplete: '.',
            right_container: Some(')'),
        }
    }
    pub fn hashes() -> Self {
        ProgressCharacterSet {
            left_container: Some('['),
            complete: '#',
            current: '>',
            incomplete: '-',
            right_container: Some(']'),
        }
    }
}

impl Default for ProgressCharacterSet {
    fn default() -> Self {
        ProgressCharacterSet::boxes()
    }
}

/// Progress bar struct
/// # Example
/// ```
/// let pb = Progress::new(10);
/// pb.advance();
/// let rendered = pb.render();
/// ```
pub struct Progress {
    completed_steps: usize,
    total_steps: usize,
    characters: ProgressCharacterSet,
}

impl Progress {
    pub fn new(total_steps: usize) -> Self {
        Progress {
            completed_steps: 0,
            total_steps,
            characters: ProgressCharacterSet::default(),
        }
    }

    pub fn with_total(mut self, total_steps: usize) -> Self {
        self.total_steps = total_steps;
        self
    }
    pub fn with_steps(mut self, completed_steps: usize) -> Self {
        self.completed_steps = completed_steps;
        self
    }
    pub fn with_progress_characters(mut self, characters: ProgressCharacterSet) -> Self {
        self.characters = characters;
        self
    }

    pub fn get_progress(&mut self) -> usize {
        self.completed_steps
    }
    pub fn get_total(&mut self) -> usize {
        self.total_steps
    }
    pub fn get_characters(&mut self) -> &ProgressCharacterSet {
        &self.characters
    }

    /// Advance the progress by one step
    ///
    /// # Example
    /// ```
    /// let mut pb = Progress::new(10);
    /// pb.advance();
    /// ```
    pub fn advance(&mut self) {
        self.completed_steps += 1;
    }

    /// Advance the progress by a specified number of steps
    /// 
    /// # Example
    /// ```
    /// let mut pb = Progress::new(20);
    /// pb.advance_by(5);
    /// ```
    pub fn advance_by(&mut self, steps: usize) {
        self.completed_steps += steps;
    }

    /// Render the progress bar as a string
    ///
    /// # Example
    /// ```
    /// let mut pb = Progress::new(0);
    /// pb.advance();
    /// let rendered = pb.render();
    /// println!("{}", rendered);
    /// ```
    ///
    /// Output:
    /// ```
    /// "[>---------]   0%"
    /// "[#>--------]  10%"
    /// ...
    /// "[##########] 100%"
    /// ```
    pub fn render(&self) -> String {
        if self.total_steps == 0 {
            return format!("{}{} {:<3}%", 
                self.characters.left_container.unwrap_or(' '), 
                self.characters.right_container.unwrap_or(' '),
                0);
        }
        let progress = (self.completed_steps * 100) / self.total_steps;
        let complete = self.completed_steps.min(self.total_steps);
        if complete == self.total_steps {
            // 100% complete: no current or incomplete chars
            format!(
                "{}{}{} 100%",
                self.characters.left_container.unwrap_or(' '),
                self.characters
                    .complete
                    .to_string()
                    .repeat(self.total_steps),
                self.characters.right_container.unwrap_or(' ')
            )
        } else {
            let incomplete = self.total_steps - complete - 1;
            format!(
                "{}{}{}{}{} {:>3}%",
                self.characters.left_container.unwrap_or(' '),
                self.characters.complete.to_string().repeat(complete),
                self.characters.current.to_string(),
                self.characters.incomplete.to_string().repeat(incomplete),
                self.characters.right_container.unwrap_or(' '),
                progress
            )
        }
    }
}

/// ====================================================================
/// Spinner

/// Defines the frames for a spinner
/// # Example
/// ```
/// let spinner = SpinnerCharacterSet::dots();
/// ```
pub struct SpinnerCharacterSet {
    pub frames: Vec<String>,
}

impl SpinnerCharacterSet {
    pub fn new(frames: Vec<String>) -> Self {
        SpinnerCharacterSet { frames }
    }
}

/// Predefined spinner character sets
impl SpinnerCharacterSet {
    pub fn dots() -> Self {
        SpinnerCharacterSet {
            frames: vec![
                "⠋".to_string(),
                "⠙".to_string(),
                "⠹".to_string(),
                "⠸".to_string(),
                "⠼".to_string(),
                "⠴".to_string(),
                "⠦".to_string(),
                "⠧".to_string(),
                "⠇".to_string(),
                "⠏".to_string(),
            ],
        }
    }
    pub fn line() -> Self {
        SpinnerCharacterSet {
            frames: vec![
                "-".to_string(),
                "\\".to_string(),
                "|".to_string(),
                "/".to_string(),
            ],
        }
    }
    pub fn circle() -> Self {
        SpinnerCharacterSet {
            frames: vec![
                "◐".to_string(),
                "◓".to_string(),
                "◑".to_string(),
                "◒".to_string(),
            ],
        }
    }
    pub fn arrow() -> Self {
        SpinnerCharacterSet {
            frames: vec![
                "←".to_string(),
                "↖".to_string(),
                "↑".to_string(),
                "↗".to_string(),
                "→".to_string(),
                "↘".to_string(),
                "↓".to_string(),
                "↙".to_string(),
            ],
        }
    }
}

impl Default for SpinnerCharacterSet {
    fn default() -> Self {
        SpinnerCharacterSet::dots()
    }
}

/// Spinner struct for indicating ongoing processes
/// # Example
/// ```
/// let spinner = Spinner::new();
/// for _ in 0..10 {
///     print!("\r{}", spinner.next_frame());
/// }
/// ```
pub struct Spinner {
    frames: SpinnerCharacterSet,
    current_frame: usize,
}

impl Spinner {
    pub fn new() -> Self {
        Spinner {
            frames: SpinnerCharacterSet::default(),
            current_frame: 0,
        }
    }

    pub fn with_frames(mut self, frames: SpinnerCharacterSet) -> Self {
        self.frames = frames;
        self
    }

    pub fn next_frame(&mut self) -> &str {
        let frame = &self.frames.frames[self.current_frame];
        self.current_frame = (self.current_frame + 1) % self.frames.frames.len();
        frame
    }
}

/// ====================================================================
/// Printer for styled output (placeholder for future implementation)

use std::{io::{self, Write}, vec};
use console::{Style, measure_text_width, strip_ansi_codes};
use indicatif::style;

pub struct BoxCorners {
    pub top_left: char,
    pub top_right: char,
    pub middle_left: char,
    pub middle_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
}

impl Default for BoxCorners {
    fn default() -> Self {
        BoxCorners {
            top_left: '┌',
            top_right: '┐',
            middle_left: '├',
            middle_right: '┤',
            bottom_left: '└',
            bottom_right: '┘',
        }
    }
}

pub struct BoxBorders {
    pub top: char,
    pub right: char,
    pub bottom: char,
    pub left: char,
}

impl Default for BoxBorders {
    fn default() -> Self {
        BoxBorders {
            top: '─',
            right: '│',
            bottom: '─',
            left: '│',
        }
    }
}

/// Styles for different output elements
/// Customize colors and box drawing characters
pub struct FormatStyle {
    pub title: Style,
    pub subtitle: Style,
    pub info: Style,
    pub warning: Style,
    pub error: Style,
    pub success: Style,

    pub corners: BoxCorners,
    pub borders: BoxBorders,
}

impl Default for FormatStyle {
    fn default() -> Self {
        FormatStyle {
            title: Style::new().bold().underlined(),
            subtitle: Style::new().bold(),
            info: Style::new().cyan(),
            warning: Style::new().yellow(),
            error: Style::new().red().bold(),
            success: Style::new().green().bold(),
            corners: BoxCorners::default(),
            borders: BoxBorders::default(),
        }
    }
}

/// Printer that writes formatted lines into any `Write`.
/// Keeps layout/measurement logic in one place.
pub struct FormattedOutputHandler<T: Write> {
    out: T,
    inner_width: usize, // visible width available for content
    formatting: FormatStyle,
    newline_on_task_complete: bool,
}

impl<T: Write> FormattedOutputHandler<T> {
    pub fn new(out: T, total_width: usize) -> Self {
        // box layout uses two border chars + two single-space paddings = 4
        let inner_width = total_width.saturating_sub(4);
        FormattedOutputHandler { out, inner_width, formatting: FormatStyle::default(), newline_on_task_complete: true }
    }

    pub fn with_width(mut self, total_width: usize) -> Self {
        self.inner_width = total_width.saturating_sub(4);
        self
    }
    pub fn with_formatting(mut self, formatting: FormatStyle) -> Self {
        self.formatting = formatting;
        self
    }
    pub fn with_newline_on_task_complete(mut self, newline: bool) -> Self {
        self.newline_on_task_complete = newline;
        self
    }

    pub fn get_inner_width(&self) -> usize {
        self.inner_width
    }
    pub fn get_formatting(&self) -> &FormatStyle {
        &self.formatting
    }

    /// Write a single centered/left-aligned styled line inside the box.
    /// style: Option<&Style> — pass None to avoid styling (useful for tests).
    /// newline: bool — if false, uses carriage return to overwrite the line.
    /// # Example
    /// ```
    /// let mut output = FormattedOutputHandler::new(&mut stdout, 80);
    /// output.line("Hello, world!", Some(&style), true);
    /// output.line("New line", Some(&style), true);
    /// ```
    /// Output:
    /// ```
    /// │ Hello, world!                                                   │
    /// │ New line                                                        │
    /// ```
    /// 
    /// If newline is false, it will overwrite the current line instead of adding a new one.
    /// # Example
    /// ```
    /// let mut output = FormattedOutputHandler::new(&mut stdout, 80);
    /// output.line("Hello, world!", Some(&style), false);
    /// output.line("New line", Some(&style), true);
    /// ```
    /// Output:
    /// ```
    /// │ New line                                                        │
    /// ```
    ///
    pub fn line(&mut self, text: &str, style: Option<&Style>, newline: bool) -> io::Result<()> {
        let rendered = if let Some(s) = style {
            s.apply_to(text).to_string()
        } else {
            text.to_string()
        };

        let visible = strip_ansi_codes(&rendered);
        let vis_w = measure_text_width(&visible);
        let pad = self.inner_width.saturating_sub(vis_w);
        if newline {
            writeln!(self.out, "{} {}{} {}", self.formatting.borders.left, rendered, " ".repeat(pad), self.formatting.borders.right)?;
        } else {
            write!(self.out, "{} {}{} {}\r", self.formatting.borders.left, rendered, " ".repeat(pad), self.formatting.borders.right)?;
            self.out.flush()?;
        }
        Ok(())
    }

    /// Write formatted text directly to the output without any box formatting.
    /// This is useful for writing raw text or progress indicators that don't fit
    /// inside the box layout.
    /// 
    /// # Example
    /// ```
    /// let mut output = FormattedOutputHandler::new(&mut stdout, 80);
    /// output.write(format_args!("Raw text without box formatting\n"));
    /// ```
    pub fn write(&mut self, args: std::fmt::Arguments) -> io::Result<()> {
        write!(self.out, "{}", args)
    }

    /// Helper to write a raw boxed inner line (already-rendered visible string).
    pub fn raw_inner(&mut self, inner_visible: &str) -> io::Result<()> {
        let vis_w = measure_text_width(&strip_ansi_codes(inner_visible));
        let pad = self.inner_width.saturating_sub(vis_w);
        writeln!(self.out, "{} {}{} {}", self.formatting.borders.left, inner_visible, " ".repeat(pad), self.formatting.borders.right)?;
        Ok(())
    }

    /// Write a full-width horizontal rule inside the box.
    pub fn hr(&mut self) -> io::Result<()> {
        writeln!(self.out, "{}{}{}", self.formatting.corners.middle_left, self.formatting.borders.top.to_string().repeat(self.inner_width + 2), self.formatting.corners.middle_right)?;
        Ok(())
    }

    /// Write the top border of the box.
    pub fn top_border(&mut self) -> io::Result<()> {
        writeln!(self.out, "{}{}{}", self.formatting.corners.top_left, self.formatting.borders.top.to_string().repeat(self.inner_width + 2), self.formatting.corners.top_right)?;
        Ok(())
    }

    /// Write the bottom border of the box.
    pub fn bottom_border(&mut self) -> io::Result<()> {
        writeln!(self.out, "{}{}{}", self.formatting.corners.bottom_left, self.formatting.borders.bottom.to_string().repeat(self.inner_width + 2), self.formatting.corners.bottom_right)?;
        Ok(())
    }

    /// Write an empty line inside the box.
    pub fn empty_line(&mut self) -> io::Result<()> {
        writeln!(self.out, "{} {} {}", self.formatting.borders.left, " ".repeat(self.inner_width), self.formatting.borders.right)?;
        Ok(())
    }

    /// Write a title line (bold/underlined) inside the box.
    pub fn title(&mut self, text: &str) -> io::Result<()> {
        let style = self.formatting.title.clone();
        self.line(text, Some(&style), true)
    }
    /// Write a subtitle line (bold) inside the box.
    pub fn subtitle(&mut self, text: &str) -> io::Result<()> {
        let style = self.formatting.subtitle.clone();
        self.line(text, Some(&style), true)
    }
    /// Write an info line (cyan) inside the box.
    pub fn info(&mut self, text: &str) -> io::Result<()> {
        let style = self.formatting.info.clone();
        self.line(text, Some(&style), true)
    }
    /// Write a warning line (yellow) inside the box.
    pub fn warning(&mut self, text: &str) -> io::Result<()> {
        let style = self.formatting.warning.clone();
        self.line(text, Some(&style), true)
    }
    /// Write an error line (red/bold) inside the box.
    pub fn error(&mut self, text: &str) -> io::Result<()> {
        let style = self.formatting.error.clone();
        self.line(text, Some(&style), true)
    }
    /// Write a success line (green/bold) inside the box.
    pub fn success(&mut self, text: &str) -> io::Result<()> {
        let style = self.formatting.success.clone();
        self.line(text, Some(&style), true)
    }
    /// Write a message line (no style) inside the box.
    pub fn message(&mut self, text: &str) -> io::Result<()> {
        self.line(text, None, true)
    }

    /// Write a frame of a progress bar inside the box.
    pub fn progress(&mut self, pb: &mut Progress, style: Option<&Style>) -> io::Result<()> {
        self.line(&pb.render(), style, &pb.get_progress() == &pb.get_total() && self.newline_on_task_complete) // newline only on complete
    }
    /// Write a frame of a spinner inside the box.
    pub fn spinner(&mut self, sp: &mut Spinner) -> io::Result<()> {
        self.line(&sp.next_frame(), None, false) // never newline
    }
    /// Write a frame of a spinner combined with a progress bar inside the box.
    pub fn spinner_with_progress(&mut self, sp: &mut Spinner, pb: &mut Progress, style: Option<&Style>) -> io::Result<()> {
        let frame = sp.next_frame();
        let combined = format!("{} {}", frame, pb.render());
        self.line(&combined, style, &pb.get_progress() == &pb.get_total() && self.newline_on_task_complete) // newline only on complete
    }
    /// Write a frame of a spinner combined with a progress bar and a message inside the box.
    pub fn spinner_with_message(&mut self, sp: &mut Spinner, message: &str, style: Option<&Style>) -> io::Result<()> {
        let frame = sp.next_frame();
        let combined = format!("{} {}", frame, message);
        self.line(&combined, style, false) // never newline
    }
    /// Write a frame of a progress bar combined with a message inside the box.
    pub fn progress_with_message(&mut self, pb: &mut Progress, message: &str, style: Option<&Style>) -> io::Result<()> {
        let combined = format!("{} {}", pb.render(), message);
        self.line(&combined, style, &pb.get_progress() == &pb.get_total() && self.newline_on_task_complete) // newline only on complete
    }
    /// Write a frame of a spinner combined with a progress bar and a message inside the box.
    pub fn spinner_and_progress_with_message(&mut self, sp: &mut Spinner, pb: &mut Progress, message: &str, style: Option<&Style>) -> io::Result<()> {
        let frame = sp.next_frame();
        let combined = format!("{} {} {}", frame, pb.render(), message);
        self.line(&combined, style, &pb.get_progress() == &pb.get_total() && self.newline_on_task_complete) // newline only on complete
    }

    /// Flush the output buffer.
    pub fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

pub type mainstage_fmt_stdout_handler = FormattedOutputHandler<std::io::Stdout>;