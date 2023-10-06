#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use std::fmt::Display;
use std::io::{self, stdout, StdoutLock, Write};

use crossterm::event::{self, Event, KeyCode, KeyEventState, KeyModifiers};
use crossterm::{cursor, queue, terminal};

/// A highlighting scheme to apply to the user input.
pub struct Highlight<'a>(pub &'a dyn Fn(&str) -> String);

/// The default characters on which to break words.
pub const WORD_BREAKS: &str = "-_=+[]{}()<>,./\\`'\";:!@#$%^&*?|~ ";

/// The result of [`Editor::read`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditResult {
    Ok(String),
    Cancel,
    Quit,
}

/// A line editor.
///
/// Ctrl-C returns an [`EditResult::Cancel`];
/// Ctrl-D returns an [`EditResult::Quit`].
///
/// Example:
/// ```no_run
/// # use linoleum::{Editor, EditResult};
/// let editor = Editor::new(" > ");
/// match editor.read().expect("Failed to read line") {
///     EditResult::Ok(s) => println!("You entered: '{s}'"),
///     EditResult::Cancel => println!("You canceled!"),
///     EditResult::Quit => std::process::exit(1),
/// }
/// ```
pub struct Editor<'a, 'b, P: Display> {
    prompt: P,
    word_breaks: &'a str,
    highlight: Highlight<'b>,
}

impl<'a, 'b, P: Display> Editor<'a, 'b, P> {
    /// Creates a new editor with empty highlight and default word breaks.
    ///
    /// Example:
    /// ```
    /// # use linoleum::Editor;
    /// let editor = Editor::new(" > ");
    /// ```
    pub fn new(prompt: P) -> Self {
        Self {
            prompt,
            word_breaks: WORD_BREAKS,
            highlight: Highlight(&|s| s.to_string()),
        }
    }

    /// Sets the word break characters the editor respects.
    ///
    /// Example:
    /// ```
    /// # use linoleum::Editor;
    /// // Create a new editor that doesn't break words.
    /// let editor = Editor::new(" > ")
    ///     .word_breaks("");
    /// ```
    pub fn word_breaks(&mut self, word_breaks: &'a str) -> &mut Self {
        self.word_breaks = word_breaks;
        self
    }

    /// Sets the highlighter of the editor.
    ///
    /// Example:
    /// ```
    /// # use linoleum::{Editor, Highlight};
    /// fn underline(s: &str, pat: &str) -> String {
    ///     // ...
    ///     # s.to_string()
    /// }
    ///
    /// // Create a new editor with a highlighter.
    /// let editor = Editor::new(" > ")
    ///     .highlight(Highlight(&|s| underline(s, "hello, world!")));
    /// ```
    pub fn highlight(&mut self, highlight: Highlight<'b>) -> &mut Self {
        self.highlight = highlight;
        self
    }

    /// Updates the prompt of the editor. Returns an [`EditResult`] with the data.
    ///
    /// Example:
    /// ```
    /// # use linoleum::{Editor, Highlight};
    /// let mut editor = Editor::new(" > ");
    /// // ...
    /// editor.prompt("{~} ");
    /// ```
    pub fn prompt(&mut self, prompt: P) -> &mut Self {
        self.prompt = prompt;
        self
    }

    /// Reads a line from stdin.
    ///
    /// Precedes with prompt. Enters terminal raw mode for the duration
    /// of the read.
    ///
    /// Example:
    /// ```no_run
    /// # use linoleum::{Editor, EditResult};
    /// let editor = Editor::new(" > ");
    /// match editor.read().expect("Failed to read line") {
    ///     EditResult::Ok(s) => println!("You entered: '{s}'"),
    ///     EditResult::Cancel => println!("You canceled!"),
    ///     EditResult::Quit => std::process::exit(1),
    /// }
    /// ```
    pub fn read(&self) -> io::Result<EditResult> {
        let mut stdout = stdout().lock();

        let prompt = self.prompt.to_string();
        let prompt_length = prompt.len();
        write!(stdout, "{}", prompt)?;
        stdout.flush()?;
        terminal::enable_raw_mode()?;

        let mut data = String::new();
        let mut cursor = 0;
        let mut cursor_line = 0;
        let mut num_lines = 0;

        loop {
            let ev = event::read();

            let ev = match ev {
                Ok(ev) => ev,
                Err(e) => {
                    terminal::disable_raw_mode()?;
                    return Err(e);
                }
            };

            if let Event::Key(key) = ev {
                let caps = key.modifiers.contains(KeyModifiers::SHIFT)
                    ^ key.state.contains(KeyEventState::CAPS_LOCK);

                match key.code {
                    KeyCode::Enter => break,
                    KeyCode::Backspace => {
                        if cursor != 0 {
                            cursor -= 1;
                            data.remove(cursor);
                            self.redraw(
                                &mut stdout,
                                &data,
                                prompt_length,
                                &mut cursor_line,
                                &mut num_lines,
                                cursor,
                            )?;
                        }
                    }
                    KeyCode::Char(mut ch) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            if ch == 'h' {
                                let old_cursor = cursor;
                                cursor = self.find_word_boundary(&data, cursor, true);

                                data = data
                                    .chars()
                                    .take(cursor)
                                    .chain(data.chars().skip(old_cursor))
                                    .collect();

                                self.redraw(
                                    &mut stdout,
                                    &data,
                                    prompt_length,
                                    &mut cursor_line,
                                    &mut num_lines,
                                    cursor,
                                )?;
                            } else if ch == 'd' {
                                terminal::disable_raw_mode()?;
                                writeln!(stdout)?;
                                return Ok(if data.is_empty() {
                                    EditResult::Quit
                                } else {
                                    EditResult::Cancel
                                });
                            } else if ch == 'c' {
                                terminal::disable_raw_mode()?;
                                writeln!(stdout)?;
                                return Ok(EditResult::Cancel);
                            }
                        } else {
                            if caps {
                                ch = ch.to_uppercase().next().unwrap();
                            }

                            data.insert(cursor, ch);
                            cursor += 1;
                            self.redraw(
                                &mut stdout,
                                &data,
                                prompt_length,
                                &mut cursor_line,
                                &mut num_lines,
                                cursor,
                            )?;
                        }
                    }
                    KeyCode::Left => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            cursor = self.find_word_boundary(&data, cursor, true);
                            self.move_to(&mut stdout, prompt_length, &mut cursor_line, cursor)?;
                        } else if cursor != 0 {
                            cursor -= 1;
                            self.move_to(&mut stdout, prompt_length, &mut cursor_line, cursor)?;
                        }
                    }
                    KeyCode::Right => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            cursor = self.find_word_boundary(&data, cursor, false) + 1;
                            self.move_to(&mut stdout, prompt_length, &mut cursor_line, cursor)?;
                        } else if cursor != data.len() {
                            cursor += 1;
                            self.move_to(&mut stdout, prompt_length, &mut cursor_line, cursor)?;
                        }
                    }
                    KeyCode::Home => {
                        cursor = 0;
                        self.move_to(&mut stdout, prompt_length, &mut cursor_line, cursor)?;
                    }
                    KeyCode::End => {
                        cursor = data.len();
                        self.move_to(&mut stdout, prompt_length, &mut cursor_line, cursor)?;
                    }
                    _ => {}
                }
            }
        }

        terminal::disable_raw_mode()?;
        writeln!(stdout)?;
        Ok(EditResult::Ok(data))
    }

    /// Finds a word boundary.
    fn find_word_boundary(&self, data: &str, start: usize, backwards: bool) -> usize {
        let chars: Vec<char> = data.chars().collect();
        let (step, stop) = if backwards {
            (-1, 0)
        } else {
            (1, data.len() as i64 - 1)
        };
        let mut i = start as i64;

        while i != stop {
            i += step;

            if self.word_breaks.contains(chars[i as usize]) {
                break;
            }
        }

        i as usize
    }

    fn move_to(
        &self,
        stdout: &mut StdoutLock,
        prompt_length: usize,
        cursor_line: &mut u16,
        end: usize,
    ) -> io::Result<()> {
        let size = terminal::size()?.0;

        let end = end + prompt_length;
        queue!(stdout, cursor::MoveToColumn(end as u16 % size as u16))?;

        let move_up = *cursor_line as i32 - end as i32 / size as i32;
        let m = move_up.unsigned_abs() as u16;
        #[allow(clippy::comparison_chain)]
        if move_up > 0 {
            queue!(stdout, cursor::MoveUp(m))?;
            *cursor_line -= m;
        } else if move_up < 0 {
            queue!(stdout, cursor::MoveDown(m))?;
            *cursor_line += m;
        }

        stdout.flush()
    }

    fn redraw(
        &self,
        stdout: &mut StdoutLock,
        data: &str,
        prompt_length: usize,
        cursor_line: &mut u16,
        num_lines: &mut u16,
        end: usize,
    ) -> io::Result<()> {
        self.clear(stdout, prompt_length, *cursor_line, *num_lines)?;

        let highlighted = (self.highlight.0)(data);
        let mut data = highlighted.as_str();

        let size = terminal::size()?.0;

        let mut cap = (size as usize - prompt_length).min(data.len());
        write!(stdout, "{}", &data[0..cap])?;

        *num_lines = 0;
        *cursor_line = 0;
        let length = data.len() + prompt_length;
        if length > size as usize {
            loop {
                data = &data[cap..];
                if data.is_empty() {
                    break;
                }

                cap = (size as usize).min(data.len());
                write!(stdout, "\r\n{}", &data[0..cap])?;
                *num_lines += 1;
                *cursor_line += 1;
            }

            let end = end + prompt_length;
            queue!(stdout, cursor::MoveToColumn((end % size as usize) as u16))?;

            let move_up = *num_lines as i32 - (end / size as usize) as i32;
            let m = move_up.unsigned_abs() as u16;
            #[allow(clippy::comparison_chain)]
            if move_up > 0 {
                queue!(stdout, cursor::MoveUp(m))?;
                *cursor_line -= m;
            } else if move_up < 0 {
                queue!(stdout, cursor::MoveDown(m))?;
                *cursor_line += m;
            }
        } else if length == size as usize && end == data.len() {
            queue!(stdout, cursor::MoveDown(1), cursor::MoveToColumn(0))?;

            *num_lines += 1;
            *cursor_line += 1;
        } else {
            queue!(stdout, cursor::MoveToColumn((end + prompt_length) as u16))?;
        }

        stdout.flush()
    }

    fn clear(
        &self,
        stdout: &mut StdoutLock,
        prompt_length: usize,
        cursor_line: u16,
        num_lines: u16,
    ) -> io::Result<()> {
        if cursor_line != 0 {
            queue!(stdout, cursor::MoveUp(cursor_line),)?;
        }

        queue!(
            stdout,
            cursor::MoveToColumn(prompt_length as u16),
            terminal::Clear(terminal::ClearType::UntilNewLine),
        )?;

        if num_lines == 0 {
            return Ok(());
        }

        for _ in 0..num_lines {
            queue!(
                stdout,
                cursor::MoveDown(1),
                cursor::MoveToColumn(0),
                terminal::Clear(terminal::ClearType::CurrentLine),
            )?;
        }

        queue!(
            stdout,
            cursor::MoveUp(num_lines),
            cursor::MoveToColumn(prompt_length as u16),
        )?;

        Ok(())
    }
}
