//! The SVG 2 path-data lexer (LCV-172 AC 1): command letters, numbers and
//! flags read from a `&str` cursor, with commas and whitespace as separators.
//! Its number reader also parses `<polyline>`/`<polygon>` `points` (LCV-174).
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

/// A cursor over one `d` attribute.
#[derive(Debug)]
pub(in crate::io::svg) struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// A cursor at the start of `d`.
    pub(in crate::io::svg) fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    /// The next command letter, consumed; `None` (nothing consumed) when the
    /// next token is not a path command.
    pub(super) fn command(&mut self) -> Option<u8> {
        self.skip_separators();
        let c = self
            .peek()
            .filter(|c| b"MmLlHhVvZzAaCcSsQqTt".contains(c))?;
        self.pos += 1;
        Some(c)
    }

    /// The next number (SVG 2 grammar: sign, digits, `.`, exponent); `None`
    /// at a syntax error or a non-finite value.
    pub(in crate::io::svg) fn number(&mut self) -> Option<f64> {
        self.skip_separators();
        let start = self.pos;
        self.eat(|c| c == b'+' || c == b'-');
        let mut digits = self.digits();
        if self.eat(|c| c == b'.') {
            digits += self.digits();
        }
        if digits == 0 {
            return None;
        }
        if self.eat(|c| c == b'e' || c == b'E') {
            self.eat(|c| c == b'+' || c == b'-');
            if self.digits() == 0 {
                return None;
            }
        }
        let value = self.src.get(start..self.pos)?.parse::<f64>().ok()?;
        value.is_finite().then_some(value)
    }

    /// The next flag: exactly one `0` or `1` character.
    pub(super) fn flag(&mut self) -> Option<bool> {
        self.skip_separators();
        let flag = match self.peek()? {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.pos += 1;
        Some(flag)
    }

    /// Whether only separators remain.
    pub(in crate::io::svg) fn at_end(&mut self) -> bool {
        self.skip_separators();
        self.peek().is_none()
    }

    /// Whether the next token starts a number (a sign, a digit or `.`).
    pub(super) fn number_ahead(&mut self) -> bool {
        self.skip_separators();
        self.peek()
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'+' | b'-' | b'.'))
    }

    fn peek(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }

    /// Consume one byte matching `pred`; whether one was consumed.
    fn eat(&mut self, pred: impl Fn(u8) -> bool) -> bool {
        let hit = self.peek().is_some_and(pred);
        self.pos += usize::from(hit);
        hit
    }

    /// Consume a run of ASCII digits; how many.
    fn digits(&mut self) -> usize {
        let from = self.pos;
        while self.eat(|c| c.is_ascii_digit()) {}
        self.pos - from
    }

    /// Skip commas and SVG whitespace (space, tab, CR, LF, form feed).
    fn skip_separators(&mut self) {
        while self.eat(|c| c == b',' || c.is_ascii_whitespace()) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbers(src: &str) -> Vec<Option<f64>> {
        let mut lx = Lexer::new(src);
        (0..src.len())
            .map_while(|_| lx.number().map(Some))
            .collect()
    }

    #[test]
    fn a_sign_or_second_dot_starts_a_new_number() {
        let mut lx = Lexer::new("M1-2.5.5");
        assert_eq!(lx.command(), Some(b'M'));
        assert_eq!(lx.number(), Some(1.0));
        assert_eq!(lx.number(), Some(-2.5));
        assert_eq!(lx.number(), Some(0.5));
        assert!(lx.at_end());
    }

    #[test]
    fn commas_and_whitespace_separate_in_any_mix() {
        let want = [1.0, 2.0, 3.0, 4.0, 5.0].map(Some).to_vec();
        assert_eq!(numbers("1,2 3\t,\n4 ,5"), want);
        assert_eq!(numbers(" \r1 , 2,3,4\x0c5 "), want);
    }

    #[test]
    fn exponents_leading_dots_and_plus_signs() {
        assert_eq!(
            numbers("1e3 -1.5E-2 .5 +2 5."),
            [1000.0, -0.015, 0.5, 2.0, 5.0].map(Some).to_vec()
        );
    }

    #[test]
    fn glued_flags_are_one_character_each() {
        let mut lx = Lexer::new("1110");
        assert_eq!(lx.flag(), Some(true));
        assert_eq!(lx.flag(), Some(true));
        assert_eq!(lx.number(), Some(10.0));
        let mut lx = Lexer::new("0,1");
        assert_eq!((lx.flag(), lx.flag()), (Some(false), Some(true)));
    }

    #[test]
    fn malformed_numbers_and_flags_are_errors() {
        for bad in [".", "1e", "-", "1e999", "-1e999", "e3", "x"] {
            assert_eq!(Lexer::new(bad).number(), None, "{bad:?}");
        }
        for bad in ["2", "-1", ".", ""] {
            assert_eq!(Lexer::new(bad).flag(), None, "{bad:?}");
        }
    }

    #[test]
    fn commands_are_path_letters_only() {
        let mut lx = Lexer::new(" ,z");
        assert_eq!(lx.command(), Some(b'z'));
        for c in "MmLlHhVvZzAaCcSsQqTt".bytes() {
            assert_eq!(Lexer::new(&char::from(c).to_string()).command(), Some(c));
        }
        let mut lx = Lexer::new("e 1");
        assert_eq!(lx.command(), None);
        assert!(!lx.at_end());
        let mut lx = Lexer::new("5");
        assert_eq!(lx.command(), None);
        assert!(lx.number_ahead());
    }
}
