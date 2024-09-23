# Linoleum

Linoleum is a smooth line editor designed roughly for a REPL experience. It's ergonomic, both on the developer side and the user side.

It supports Ctrl-C/-D/-Left/-Right/-Backspace, all out of the box. The characters used by the latter three to delimit words are fully configurable.

Supports history. Make sure to run [`Editor::save_history`] before dropping the `History`.

Also supports completion with a similar interface to prompts; see [`Editor::completion`]. Note that completions only respect spaces, not the usual word breaks; this is because some (i.e. file) completions may require more license.

## Examples

```rust,no_run
use linoleum::{Editor, EditResult};

fn main() {
    let mut editor = Editor::new(" > ");
    match editor.read().expect("Failed to read line") {
        EditResult::Ok(s) => println!("You entered: '{s}'"),
        EditResult::Cancel => println!("You canceled!"),
        EditResult::Quit => std::process::exit(1),
    }
}
```

```rust,no_run
use std::fmt;
use linoleum::{Editor, EditResult};

struct Prompt {
    template: String,
}

impl fmt::Display for Prompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.template.replace("{greet}", "hello"))
    }
}

fn main() {
    let prompt = Prompt { template: " {greet}> ".to_string() };
    let mut editor = Editor::new(prompt)
        .highlight(
            |data| data.replace("foo", "bar")
        );

    loop {
        match editor.read() {
            Err(e) => {
                eprintln!("failed to read line: {e}");
                break;
            }

            Ok(EditResult::Ok(s)) => {
                if s == "exit" {
                    break;
                } else if s == "clear" {
                    print!("{}[2J{0}[0;0H", 27 as char);
                } else {
                    eprintln!("huh?");
                }
            }

            Ok(EditResult::Cancel) => continue,
            Ok(EditResult::Quit) => break,
        }
    }
}
```

