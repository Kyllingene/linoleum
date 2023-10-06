# Linoleum

Linoleum is a line editor designed for use in the [`gosh` shell](https://github.com/Kyllingene/gosh). It's ergonomic, both on the developer side and the user side.

It supports Ctrl-C/-D/-Left/-Right/-Backspace, all out of the box. The characters used to break words by the latter three are fully configurable.

History is not yet implemented, but is a top priority (since it's being used in a shell). Completion will also be supported via configurable completion function, similar to how prompts work now.

## Examples

```rust
use linoleum::{Editor, EditResult};

fn main() {
    let editor = Editor::new(" > ");
    match editor.read().expect("Failed to read line") {
        EditResult::Ok(s) => println!("You entered: '{s}'"),
        EditResult::Cancel => println!("You canceled!"),
        EditResult::Quit => std::process::exit(1),
    }
}
```

```rust
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
    let editor = Editor::new(prompt);

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

