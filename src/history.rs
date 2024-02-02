use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};

#[derive(Debug, Clone)]
pub struct History {
    lines: Vec<String>,
    file: String,
    index: usize,
    max_lines: usize,
}

impl History {
    /// Creates a new history. Reads from the provided file,
    /// if it exists.
    pub fn new(file_path: String, max_lines: usize) -> io::Result<Self> {
        let mut lines = String::new();

        match File::open(&file_path) {
            Ok(mut f) => {
                f.read_to_string(&mut lines)?;
            }
            Err(e) => {
                if e.kind() != ErrorKind::NotFound {
                    return Err(e);
                }
            }
        }

        let lines: Vec<String> = lines.lines().map(str::to_string).collect();

        Ok(Self {
            index: lines.len(),
            lines,
            file: file_path,
            max_lines,
        })
    }

    /// Save the history to the file, creating it if
    /// it doesn't exist.
    pub fn save(&self) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .truncate(true)
            .write(true)
            .create(true)
            .open(&self.file)?;

        write!(file, "{}", self.lines.join("\n"))
    }

    /// Adds a line to the history.
    pub fn push(&mut self, l: String) {
        self.lines.push(l);
        self.lines.truncate(self.max_lines);
        self.index = self.lines.len();
    }

    /// Resets the index.
    pub(crate) fn reset_index(&mut self) {
        self.index = self.lines.len();
    }

    /// Go up in the history one line, if possible.
    pub(crate) fn up(&mut self) -> Option<String> {
        if self.index > 0 {
            self.index -= 1;
            self.lines.get(self.index).cloned()
        } else {
            None
        }
    }

    /// Go down in the history one line, if possible.
    pub(crate) fn down(&mut self) -> Option<String> {
        if self.index + 1 < self.lines.len() {
            self.index += 1;
            self.lines.get(self.index).cloned()
        } else {
            None
        }
    }
}
