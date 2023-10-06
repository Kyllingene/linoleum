use std::fmt::Display;
use std::io::{self, stdout, StdoutLock, Write};

use crossterm::event::{self, Event, KeyCode, KeyEventState, KeyModifiers};
use crossterm::{cursor, queue, terminal, ExecutableCommand};

/// A highlighting scheme to apply to the user input.
pub struct Highlight<'a>(pub &'a dyn Fn(&str) -> String);

/// The default characters on which to break words.
pub const WORD_BREAKS: &str = "-_=+[]{}()<>,./\\`'\";:!@#$%^&*?|~ ";

/// The result of [`Linefeed::read`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineResult {
    Ok(String),
    Cancel,
    Quit,
}

/// A line reader.
pub struct Linefeed<'a, 'b, P: Display> {
    prompt: P,
    word_breaks: &'a str,
    highlight: Highlight<'b>,
}

impl<'a, 'b, P: Display> Linefeed<'a, 'b, P> {
    /// Creates a new linefeed with empty highlight and default word breaks.
    pub fn new(prompt: P) -> Self {
        Self {
            prompt,
            word_breaks: WORD_BREAKS,
            highlight: Highlight(&|s| s.to_string()),
        }
    }

    /// Sets the word break characters the linefeed respects.
    pub fn word_breaks(&mut self, word_breaks: &'a str) -> &mut Self {
        self.word_breaks = word_breaks;
        self
    }

    /// Sets the highlight of the linefeed.
    pub fn highlight(&mut self, highlight: Highlight<'b>) -> &mut Self {
        self.highlight = highlight;
        self
    }

    /// Reads a line from stdin.
    ///
    /// Precedes with prompt. Enters terminal raw mode for the duration
    /// of the read.
    pub fn read(&mut self) -> io::Result<LineResult> {
        let mut stdout = stdout().lock();

        let prompt = self.prompt.to_string();
        let prompt_length = prompt.len();
        write!(stdout, "{}", prompt)?;
        stdout.flush()?;
        terminal::enable_raw_mode()?;

        let mut data = String::new();
        let mut cursor = 0;
        let mut cursor_line = 0;
        let mut lines = 0;

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
                            (cursor_line, lines) = self.redraw(
                                &mut stdout,
                                &data,
                                cursor_line,
                                lines,
                                prompt_length,
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

                                (cursor_line, lines) = self.redraw(
                                    &mut stdout,
                                    &data,
                                    cursor_line,
                                    lines,
                                    prompt_length,
                                    cursor,
                                )?;
                            } else if ch == 'd' {
                                terminal::disable_raw_mode()?;
                                writeln!(stdout)?;
                                return Ok(if data.is_empty() {
                                    LineResult::Quit
                                } else {
                                    LineResult::Cancel
                                });
                            } else if ch == 'c' {
                                terminal::disable_raw_mode()?;
                                writeln!(stdout)?;
                                return Ok(LineResult::Cancel);
                            }
                        } else {
                            if caps {
                                ch = ch.to_uppercase().next().unwrap();
                            }

                            data.insert(cursor, ch);
                            cursor += 1;
                            (cursor_line, lines) = self.redraw(
                                &mut stdout,
                                &data,
                                cursor_line,
                                lines,
                                prompt_length,
                                cursor,
                            )?;
                        }
                    }
                    KeyCode::Left => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            let old_cursor = cursor;
                            cursor = self.find_word_boundary(&data, cursor, true);
                            stdout.execute(cursor::MoveLeft((old_cursor - cursor) as u16))?;
                        } else if cursor != 0 {
                            cursor -= 1;
                            stdout.execute(cursor::MoveLeft(1))?;
                        }
                    }
                    KeyCode::Right => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            let old_cursor = cursor;
                            cursor = self.find_word_boundary(&data, cursor, false) + 1;
                            stdout.execute(cursor::MoveRight((cursor - old_cursor) as u16))?;
                        } else if cursor != data.len() {
                            cursor += 1;
                            stdout.execute(cursor::MoveRight(1))?;
                        }
                    }
                    _ => {}
                }
            }
        }

        terminal::disable_raw_mode()?;
        writeln!(stdout)?;
        Ok(LineResult::Ok(data))
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

    /// Redraws the user input.
    ///
    /// Returns the new cursor_line and num_lines.
    fn redraw(
        &self,
        stdout: &mut StdoutLock,
        data: &str,
        cursor_line: u16,
        num_lines: u16,
        prompt_length: usize,
        end: usize,
    ) -> io::Result<(u16, u16)> {
        let size = terminal::size()?.0;

        let cap = data.len().min(size as usize - prompt_length);
        let first_line = &data[0..cap];

        let mut lines = Vec::with_capacity((data.len() + prompt_length) / size as usize);
        if lines.capacity() > 0 {
            let mut data = &data[size as usize - prompt_length..];

            while !data.is_empty() {
                let (chunk, rest) = data.split_at((size as usize).min(data.len()));
                lines.push(chunk);
                data = rest;
            }
        }

        self.clear(stdout, prompt_length, cursor_line, num_lines)?;
        write!(stdout, "{first_line}")?;

        for line in &lines {
            write!(stdout, "\r\n{line}")?;
        }

        queue!(
            stdout,
            cursor::MoveToColumn(((end + prompt_length) % size as usize) as u16),
        )?;

        let go_up = (lines.len() - ((end + prompt_length - 1) / size as usize)) as u16;
        if go_up != 0 {
            queue!(stdout, cursor::MoveUp(go_up))?;
        }

        stdout.flush()?;

        Ok(((end / size as usize) as u16, lines.len() as u16))
    }

    /// Clears the user input from the screen.
    fn clear(
        &self,
        stdout: &mut StdoutLock,
        prompt_length: usize,
        cursor_line: u16,
        lines: u16,
    ) -> io::Result<()> {
        if cursor_line != 0 {
            queue!(stdout, cursor::MoveUp(cursor_line))?;
        }

        queue!(
            stdout,
            cursor::MoveToColumn(prompt_length as u16),
            terminal::Clear(terminal::ClearType::UntilNewLine),
        )?;

        if cursor_line == 0 {
            return stdout.flush();
        }

        queue!(stdout, cursor::MoveToColumn(0))?;

        for _ in 0..lines {
            queue!(
                stdout,
                cursor::MoveDown(1),
                terminal::Clear(terminal::ClearType::CurrentLine),
            )?;
        }

        if lines != 0 {
            queue!(stdout, cursor::MoveUp(lines))?;
        }

        queue!(
            stdout,
            cursor::MoveToColumn(prompt_length as u16),
        )?;

        stdout.flush()
    }
}
