use std::io::{self, Write, stdout};
use std::fmt::Display;

use crossterm::{terminal, ExecutableCommand, cursor};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

/// The default characters on which to break words.
pub const WORD_BREAKS: &str = "-_=+[]{}()<>,./\\`'\";:!@#$%^&*?|~ ";

/// The line reader. 
pub struct Linefeed<'a, P: Display> {
    prompt: P,
    word_breaks: &'a str,
}

impl<'a, P: Display> Linefeed<'a, P> {
    /// Creates a new linefeed.
    pub fn new(prompt: P, word_breaks: &'a str) -> Self {
        Self {
            prompt,
            word_breaks,
        }
    }

    /// Creates a new linefeed. Uses default word break characters.
    pub fn default(prompt: P) -> Self {
        Self {
            prompt,
            word_breaks: WORD_BREAKS,
        }
    }

    /// Reads a line from stdin.
    ///
    /// Precedes with prompt. Enters terminal raw mode for the duration
    /// of the read.
    pub fn read(&mut self) -> io::Result<String> {
        let mut stdout = stdout().lock();

        print!("{}", self.prompt);
        stdout.flush()?;
        terminal::enable_raw_mode()?;

        let mut data = String::new();
        let mut cursor = 0;

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
                match key.code {
                    KeyCode::Enter => break,
                    KeyCode::Backspace => {
                        if cursor != 0 {
                            stdout.execute(cursor::MoveLeft(1))?;
                            write!(stdout, " ")?;
                            stdout.execute(cursor::MoveLeft(1))?;

                            cursor -= 1;
                            data.truncate(cursor);
                        }
                    }
                    KeyCode::Char(ch) => {
                        if ch == 'h' && key.modifiers.contains(KeyModifiers::CONTROL) {
                            let old_cursor = cursor;
                            cursor = self.find_word_boundary(&data, cursor, true);
                            stdout.execute(cursor::MoveLeft((old_cursor - cursor) as u16))?;
                            let diff = old_cursor - cursor;
                            write!(stdout, "{}", " ".repeat(diff))?;
                            stdout.execute(cursor::MoveLeft(diff as u16))?;

                            data.truncate(cursor);
                        } else {
                            write!(stdout, "{ch}")?;
                            stdout.flush()?;

                            data.insert(cursor, ch);
                            cursor += 1;

                            if cursor != data.len() {
                                write!(stdout, "{}", &data[cursor..])?;
                                stdout.execute(cursor::MoveLeft((data.len() - cursor) as u16))?;
                                stdout.flush()?;
                            }
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
        println!();
        Ok(data)
    }

    /// Finds a word boundary.
    fn find_word_boundary(&self, data: &str, start: usize, backwards: bool) -> usize {
        let chars: Vec<char> = data.chars().collect();
        let (step, stop) = if backwards { ( -1, 0 ) } else { ( 1, data.len() as i64 - 1 ) };
        let mut i = start as i64;

        while i != stop {
            i += step;

            if self.word_breaks.contains(chars[i as usize]) {
                break;
            }
        }

        i as usize
    }
}

// impl<'a, P: Display> Drop for Linefeed<'a, P> {
//     fn drop(&mut self) {
//         terminal::disable_raw_mode()
//             .expect("failed to disable raw mode");
//     }
// }

