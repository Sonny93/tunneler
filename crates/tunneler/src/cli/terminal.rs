//! The terminal the user types answers on: stdin for text, the TTY for hidden
//! input, stderr for questions.

use std::io::{self, BufRead, IsTerminal, Write};

use zeroize::Zeroizing;

/// Whether the command may ask the user something: stdin and stderr are both
/// terminals.
pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

/// Where questions are asked and answers read.
pub trait Terminal: Send + Sync {
    /// Shows a message without asking anything.
    fn write(&self, text: &str);

    /// Asks for text that must not be displayed while typed.
    fn read_secret(&self, prompt: &str) -> io::Result<Zeroizing<String>>;

    /// Asks for a line of text.
    fn read_line(&self, prompt: &str) -> io::Result<String>;
}

/// The real terminal.
#[derive(Debug, Clone, Copy, Default)]
pub struct StandardTerminal;

impl Terminal for StandardTerminal {
    fn write(&self, text: &str) {
        eprintln!("{text}");
    }

    fn read_secret(&self, prompt: &str) -> io::Result<Zeroizing<String>> {
        write_prompt(prompt)?;
        let secret = rpassword::read_password()?;
        Ok(Zeroizing::new(secret))
    }

    fn read_line(&self, prompt: &str) -> io::Result<String> {
        write_prompt(prompt)?;
        let mut line = String::new();
        io::stdin().lock().read_line(&mut line)?;
        Ok(line.trim_end_matches(['\r', '\n']).to_owned())
    }
}

fn write_prompt(prompt: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    write!(stderr, "{prompt}")?;
    stderr.flush()
}
