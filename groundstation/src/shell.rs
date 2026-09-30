pub const MAX_LINES: usize = 500;
pub const CHUNK: usize = 70;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    lines: Vec<Vec<char>>,
    incoming: Vec<u8>,
    cursor_x: usize,
    cursor_y: usize,
    home: Option<usize>,
}

impl Default for Shell {
    fn default() -> Self {
        Shell { lines: Vec::new(), incoming: Vec::new(), cursor_x: 0, cursor_y: 0, home: None }
    }
}

impl Shell {
    pub fn lines(&self) -> Vec<String> {
        self.lines.iter().map(|line| line.iter().collect()).collect()
    }

    pub fn command_sent(&mut self) {
        self.home = None;
    }

    pub fn chunks(command: &str) -> Vec<Vec<u8>> {
        let output = format!("{command}\n").into_bytes();
        output.chunks(CHUNK).map(<[u8]>::to_vec).collect()
    }

    pub fn receive(&mut self, data: &[u8]) {
        self.incoming.extend_from_slice(data);
        while !self.incoming.is_empty() {
            let newline = self.incoming.iter().position(|b| *b == b'\n');
            let idx = newline.unwrap_or(self.incoming.len());
            let mut fragment = self.incoming[..idx].to_vec();
            if !self.process_ansi(&mut fragment) {
                return;
            }
            let text: Vec<char> = String::from_utf8_lossy(&fragment).chars().collect();
            self.write_line(self.cursor_y, &text);
            if newline.is_some() {
                self.cursor_y += 1;
                self.cursor_x = 0;
                if self.cursor_y >= self.lines.len() {
                    self.lines.resize(self.cursor_y + 1, Vec::new());
                }
            }
            self.incoming.drain(..idx + usize::from(newline.is_some()));
        }
    }

    fn process_ansi(&mut self, line: &mut Vec<u8>) -> bool {
        let mut i = 0;
        while i < line.len() {
            if line[i] != 0x1B {
                i += 1;
                continue;
            }
            if i + 2 >= line.len() || line[i + 1] != b'[' {
                return false;
            }
            match line[i + 2] {
                b'H' => match self.home {
                    None => self.home = Some(self.cursor_y),
                    Some(home) => {
                        self.cursor_y = home;
                        self.cursor_x = 0;
                    }
                },
                b'K' => {
                    let erase = self.cursor_x + i;
                    if let Some(row) = self.lines.get_mut(self.cursor_y).filter(|row| erase < row.len()) {
                        row.truncate(erase);
                    }
                }
                b'2' => {
                    if i + 3 >= line.len() {
                        return false;
                    }
                    if let Some(home) = self.home.filter(|_| line[i + 3] == b'J') {
                        self.lines.iter_mut().skip(home).for_each(Vec::clear);
                    }
                    line.remove(i + 3);
                }
                _ => {
                    i += 1;
                    continue;
                }
            }
            line.drain(i..i + 3);
        }
        true
    }

    fn write_line(&mut self, line: usize, text: &[char]) {
        if line >= self.lines.len() {
            self.lines.resize(line + 1, Vec::new());
        }
        let excess = self.lines.len().saturating_sub(MAX_LINES);
        self.lines.drain(..excess);
        let line = line - excess;
        self.cursor_y -= excess;
        self.home = self.home.and_then(|home| home.checked_sub(excess));
        let row = &mut self.lines[line];
        if self.cursor_x <= row.len() {
            let end = (self.cursor_x + text.len()).min(row.len());
            row.splice(self.cursor_x..end, text.iter().copied());
        }
        self.cursor_x += text.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_splits_on_newlines_and_leaves_the_row_being_assembled() {
        let mut shell = Shell::default();
        shell.receive(b"nsh> ver\r\nHW arch: PX4\n");
        shell.receive(b"nsh> ");
        assert_eq!(shell.lines(), vec!["nsh> ver\r", "HW arch: PX4", "nsh> "]);
    }

    #[test]
    fn an_escape_split_across_packets_waits_for_the_rest() {
        let mut shell = Shell::default();
        shell.receive(b"abcdef\x1b");
        assert_eq!(shell.lines(), Vec::<String>::new(), "a fragment ending inside an escape writes nothing yet");
        shell.receive(b"[Kxy\n");
        assert_eq!(shell.lines(), vec!["abcdefxy", ""]);
    }

    #[test]
    fn home_then_clear_rewinds_and_blanks_from_home_down() {
        let mut shell = Shell::default();
        shell.receive(b"top\n\x1b[Hone\ntwo\n");
        shell.receive(b"\x1b[H\x1b[2Jnew\n");
        assert_eq!(shell.lines(), vec!["top", "new", "", ""], "the second home rewinds to row 1 and 2J blanks every row from it");
    }

    #[test]
    fn erase_to_end_of_line_counts_the_escape_position_as_qt_does() {
        let mut shell = Shell::default();
        shell.receive(b"0123456789\n");
        shell.receive(b"\x1b[H");
        shell.receive(b"ab\x1b[K\n");
        assert_eq!(shell.lines()[1], "ab", "K erases from cursor_x plus the escape's offset in the fragment");
    }

    #[test]
    fn the_history_keeps_the_last_500_rows() {
        let mut shell = Shell::default();
        (0..600).for_each(|n| shell.receive(format!("{n}\n").as_bytes()));
        let lines = shell.lines();
        assert_eq!((lines.len(), lines[0].as_str(), lines[MAX_LINES - 1].as_str(), lines[MAX_LINES].as_str()), (MAX_LINES + 1, "100", "599", ""), "the trim runs on a write, so the row a newline opens sits one past the cap as it does in the model");
    }

    #[test]
    fn a_command_goes_out_in_70_byte_chunks_ending_with_a_newline() {
        let long = "x".repeat(75);
        let chunks = Shell::chunks(&long);
        assert_eq!((chunks.len(), chunks[0].len(), chunks[1].as_slice()), (2, 70, b"xxxxx\n".as_slice()));
    }
}
