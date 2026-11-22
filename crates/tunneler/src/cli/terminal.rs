//! The terminal the user types answers on: stdin for text, the TTY for hidden
//! input, stderr for questions.

use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, IsTerminal, Write};

use zeroize::Zeroizing;

/// Whether the command may ask the user something: stdin and stderr are both
/// terminals.
pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

/// The file that stands for the terminal the user types on, whatever stdin
/// carries.
const CONTROLLING_TERMINAL_PATH: &str = if cfg!(windows) { "CONIN$" } else { "/dev/tty" };

/// Whether the command may ask the user something even when stdin carries
/// data: stderr is a terminal and the controlling terminal can be opened.
pub fn can_ask_on_controlling_terminal() -> bool {
    io::stderr().is_terminal() && open_controlling_terminal().is_ok()
}

fn open_controlling_terminal() -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(cfg!(windows))
        .open(CONTROLLING_TERMINAL_PATH)
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
        read_answer(&mut io::stdin().lock())
    }
}

/// The terminal of the user as the controlling terminal: questions on stderr,
/// answers from `/dev/tty` (`CONIN$` on Windows), so that it works when stdin
/// is the file being read.
#[derive(Debug, Clone, Copy, Default)]
pub struct ControllingTerminal;

impl Terminal for ControllingTerminal {
    fn write(&self, text: &str) {
        eprintln!("{text}");
    }

    fn read_secret(&self, _prompt: &str) -> io::Result<Zeroizing<String>> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    fn read_line(&self, prompt: &str) -> io::Result<String> {
        write_prompt(prompt)?;
        read_answer(&mut BufReader::new(open_controlling_terminal()?))
    }
}

fn read_answer(reader: &mut impl BufRead) -> io::Result<String> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}

fn write_prompt(prompt: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    write!(stderr, "{prompt}")?;
    stderr.flush()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn should_read_one_line_without_its_line_ending_when_reading_an_answer() {
        let mut input = Cursor::new("yes\r\nnext\n");

        assert_eq!(read_answer(&mut input).unwrap(), "yes");
    }

    #[test]
    fn should_read_an_empty_answer_when_the_input_ends() {
        let mut input = Cursor::new("");

        assert_eq!(read_answer(&mut input).unwrap(), "");
    }

    #[test]
    fn should_not_hand_out_secrets_when_reading_from_the_controlling_terminal() {
        let outcome = ControllingTerminal.read_secret("passphrase: ");

        assert_eq!(outcome.unwrap_err().kind(), io::ErrorKind::Unsupported);
    }
}
