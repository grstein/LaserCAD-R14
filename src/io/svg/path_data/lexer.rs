//! The SVG 2 path-data lexer (LCV-172 AC 1): command letters, numbers and
//! flags read from a `&str` cursor, with commas and whitespace as separators.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

/// A cursor over one `d` attribute.
#[derive(Debug)]
pub(super) struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// A cursor at the start of `d`.
    pub(super) fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    /// The next command letter, consumed; `None` (nothing consumed) when the
    /// next token is not a path command.
    pub(super) fn command(&mut self) -> Option<u8> {
        let _ = (self.src, self.pos);
        None
    }

    /// The next number; `None` at a syntax error or a non-finite value.
    pub(super) fn number(&mut self) -> Option<f64> {
        None
    }

    /// The next flag: exactly one `0` or `1` character.
    pub(super) fn flag(&mut self) -> Option<bool> {
        None
    }

    /// Whether only separators remain.
    pub(super) fn at_end(&mut self) -> bool {
        false
    }

    /// Whether the next token starts a number (a sign, a digit or `.`).
    pub(super) fn number_ahead(&mut self) -> bool {
        false
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
