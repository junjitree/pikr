//! dmenu mode — stdin in, stdout out.

use super::{Entry, Mode};
use anyhow::Result;
use std::io::{self, BufRead, IsTerminal};
use std::sync::{Arc, mpsc};

#[derive(Default)]
pub struct Dmenu;

impl Mode for Dmenu {
    fn collect(&mut self) -> Result<Vec<Entry>> {
        require_piped_stdin()?;
        Ok(read_entries(io::stdin().lock())?)
    }
}

fn require_piped_stdin() -> Result<()> {
    if io::stdin().is_terminal() {
        anyhow::bail!("dmenu mode requires entries on stdin");
    }
    Ok(())
}

/// One entry per non-empty line.
pub(crate) fn read_entries(reader: impl BufRead) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if !line.is_empty() {
            entries.push(Entry::stdout(line));
        }
    }
    Ok(entries)
}

/// `--loading`: read stdin on a background thread so the window can open
/// before the producer finishes. The entries arrive once, when stdin closes.
pub fn read_stdin_in_background() -> Result<mpsc::Receiver<Arc<Vec<Entry>>>> {
    require_piped_stdin()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let entries = read_entries(io::stdin().lock()).unwrap_or_else(|e| {
            eprintln!("pikr: reading stdin: {e}");
            Vec::new()
        });
        let _ = tx.send(Arc::new(entries));
    });
    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(input: &str) -> Vec<String> {
        read_entries(io::Cursor::new(input))
            .unwrap()
            .into_iter()
            .map(|e| e.label)
            .collect()
    }

    #[test]
    fn one_entry_per_line() {
        assert_eq!(labels("a\nb\nc\n"), ["a", "b", "c"]);
    }

    #[test]
    fn skips_empty_lines_and_handles_missing_final_newline() {
        assert_eq!(labels("a\n\n\nb"), ["a", "b"]);
    }

    #[test]
    fn empty_input_is_no_entries() {
        assert!(labels("").is_empty());
    }

    #[test]
    fn invalid_utf8_is_an_error() {
        assert!(read_entries(io::Cursor::new(b"ok\n\xff\xfe\n".to_vec())).is_err());
    }
}
