use std::{
    collections::{BTreeMap, BTreeSet},
    num::{ParseFloatError, ParseIntError},
};

use tracing::trace;

// The end index is non inclusive.. I.E. it is up to, but not including the
// end index. A token that is of size one, should have an end index one larger
// than the start index.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub line: u32,
    pub index: u32,
}

impl Location {
    pub fn new(line: u32, index: u32) -> Self {
        Location { line, index }
    }
}

impl Default for Location {
    fn default() -> Self {
        Self { line: 0, index: 0 }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TokenRange {
    /// Starting character in a line (inclusive)
    start: u32,
    /// Ending character in a line (exclusive)
    end: u32,
}

impl TokenRange {
    /// Create a new `TokenRange`. Returns `None` if `start` is not less than `end`.
    pub fn new(start: u32, end: u32) -> Option<Self> {
        if start < end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// Create a new `TokenRange` from a start and end `Location`. The `start`
    /// and `end` must be on the same line and `start` must be less than `end`.
    pub fn new_line(start: Location, end: Location) -> Option<Self> {
        if start.line != end.line {
            return None;
        }
        return Self::new(start.index, end.index);
    }

    /// Get the length of the `TokenRange`
    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Checks if this range overlaps with the `other` `TokenRange`.
    pub fn overlaps(&self, other: &TokenRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Finds the intersection between two ranges.
    pub fn intersection(&self, other: &TokenRange) -> Option<TokenRange> {
        if self.overlaps(other) {
            TokenRange::new(self.start.max(other.start), self.end.min(other.end))
        } else {
            None
        }
    }

    /// Finds the differences between two ranges. The result is subtracting the
    /// `other` TokenRange from this TokenRange.
    pub fn difference(&self, other: &TokenRange) -> Vec<TokenRange> {
        let mut rtn = Vec::new();
        match self.partial_cmp(other) {
            Some(core::cmp::Ordering::Less | core::cmp::Ordering::Greater) => {
                rtn.push(self.clone())
            }
            None => {
                if self.start < other.start {
                    rtn.push(TokenRange {
                        start: self.start,
                        end: other.start,
                    });
                }

                if self.end > other.end {
                    rtn.push(TokenRange {
                        start: other.end,
                        end: self.end,
                    });
                }
            }
            _ => {}
        }
        return rtn;
    }
}

impl PartialOrd for TokenRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.start < other.start && self.end <= other.start {
            Some(core::cmp::Ordering::Less)
        } else if self.start >= other.end && self.end > other.end {
            Some(core::cmp::Ordering::Greater)
        } else if self.start == other.start && self.end == other.end {
            Some(core::cmp::Ordering::Equal)
        } else {
            None
        }
    }
}

mod token_map {
    use super::{Location, TokenRange};
    use std::collections::{BTreeMap, BTreeSet};

    // NOTE: Implement `Ord` inside of the `token_map` module so we don't
    // expose it to the public.
    impl Ord for crate::lexers::TokenRange {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            match self.partial_cmp(other) {
                Some(ord) => ord,
                None => std::cmp::Ordering::Equal,
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TokenMap<T: Clone> {
        pub(crate) line_map: BTreeMap<u32, BTreeMap<TokenRange, T>>,
    }
    impl<T: Clone> TokenMap<T> {
        // Create a new instance of a TokenMap.
        pub fn new() -> Self {
            Self {
                line_map: BTreeMap::new(),
            }
        }

        /// Clear the tokens
        pub fn clear(&mut self) {
            self.line_map.clear();
        }

        pub fn insert(&mut self, line: u32, range: TokenRange, data: T) -> bool {
            let tokens = match self.line_map.get_mut(&line) {
                Some(tokens) => tokens,
                None => {
                    // If we don't have any tokens in the current line, just
                    // insert a new Vec containing the current range and data.
                    let mut tokens = BTreeMap::new();
                    tokens.insert(range, data);
                    self.line_map.insert(line, tokens);
                    return true;
                }
            };

            // Now the fun begins. We need to loop through all tokens to find the
            // ranges that overlap with our new range.

            let mut ranges: BTreeSet<TokenRange> = BTreeSet::new();
            let mut test_range = range;
            let mut keys = tokens.keys().peekable();
            while let Some(key) = keys.next() {
                match test_range.partial_cmp(key) {
                    Some(core::cmp::Ordering::Less) => {
                        ranges.insert(test_range);
                        break;
                    }
                    Some(core::cmp::Ordering::Greater) => {
                        // Check if this is the last key
                        if keys.peek().is_none() {
                            ranges.insert(test_range);
                            break;
                        }
                    }
                    Some(core::cmp::Ordering::Equal) => {
                        // test_range is already equal to a key in the map. Don't inset.
                        break;
                    }
                    None => {
                        // We've reached an overlap
                        let differences = test_range.difference(key);
                        match differences.len() {
                            2 => {
                                ranges.insert(differences[0]);
                                // If this is the last item, insert both ranges.
                                // Otherwise, update the test_range to be the
                                // greater range and continue.
                                if keys.peek().is_none() {
                                    ranges.insert(differences[1]);
                                    break;
                                } else {
                                    test_range = differences[1];
                                    continue;
                                }
                            }
                            1 => {
                                // Difference resulting in one new range. Need to check if
                                // the new range is before or after the current key.
                                match &differences[0].partial_cmp(key) {
                                    Some(core::cmp::Ordering::Less) => {
                                        ranges.insert(differences[0]);
                                        break;
                                    }
                                    Some(core::cmp::Ordering::Greater) => {
                                        if keys.peek().is_none() {
                                            ranges.insert(differences[0]);
                                            break;
                                        } else {
                                            test_range = differences[0];
                                        }
                                    }
                                    _ => {
                                        // Should never get here
                                        panic!(
                                            "TokenMap reached unexpected condition while inserting"
                                        );
                                        break;
                                    }
                                }
                            }
                            _ => {
                                // Should never get here
                                panic!("TokenMap reached unexpected condition while inserting");
                                break;
                            }
                        }
                    }
                }
            }

            if ranges.is_empty() {
                return false;
            }

            for r in ranges {
                tokens.insert(r, data.clone());
            }

            return true;
        }

        /// Insert a new token with the range between `start` and `end`. The
        /// `end` location is expected to be exclusive. Both `start` and
        /// `end` must be on the same line.
        ///
        /// This method handles splitting the range so it does not conflict
        /// with existing tokens. It gives precedence to the existing tokens.
        /// It will *NOT* merge the token data since this structure does not
        /// know the data type.
        ///
        /// Returns `true` if the item was inserted; false otherwise.
        pub fn insert_location(&mut self, start: Location, end: Location, data: T) -> bool {
            if start.line != end.line {
                return false;
            }

            let range = match TokenRange::new(start.index, end.index) {
                Some(r) => r,
                None => return false,
            };

            return self.insert(start.line, range, data);
        }

        /// Gets an iterator for the `TokenMap`
        pub fn iter(&self) -> TokenMapIterator<T> {
            return TokenMapIterator::new(self);
        }

        /// Get an iterator for the `TokenMap` where each token location is
        /// relative to the previous token.
        pub fn relative_iter(&self) -> RelativeTokenMapIterator<T> {
            return RelativeTokenMapIterator::new(self);
        }

        /// Get an iterator for the specified `line`. Returns `None` if there
        /// are not any tokens for the specified `line`.
        pub fn line_iter(
            &self,
            line: u32,
        ) -> Option<std::collections::btree_map::Iter<TokenRange, T>> {
            let current_line = self.line_map.get(&line)?;
            Some(current_line.iter())
        }
    }

    pub struct TokenMapIterator<'a, T: Clone> {
        line_iter: std::collections::btree_map::Iter<'a, u32, BTreeMap<TokenRange, T>>,
        current_line: Option<(&'a u32, &'a BTreeMap<TokenRange, T>)>,
        token_iter: Option<std::collections::btree_map::Iter<'a, TokenRange, T>>,
    }

    impl<'a, T: Clone> TokenMapIterator<'a, T> {
        fn new(token_map: &'a TokenMap<T>) -> Self {
            let mut line_iter = token_map.line_map.iter();
            let current_line = line_iter.next();
            let token_iter = match current_line {
                Some(current_line) => Some(current_line.1.iter()),
                None => None,
            };
            Self {
                line_iter,
                current_line,
                token_iter,
            }
        }
    }
    impl<'a, T: Clone> Iterator for TokenMapIterator<'a, T> {
        type Item = (&'a u32, &'a TokenRange, &'a T);

        fn next(&mut self) -> Option<Self::Item> {
            let current_line = match self.current_line {
                Some(current_line) => current_line,
                None => return None,
            };

            let token_iter = match &mut self.token_iter {
                Some(token_iter) => token_iter,
                None => {
                    // Update the token_iter
                    self.token_iter = Some(current_line.1.iter());
                    match &mut self.token_iter {
                        Some(token_iter) => token_iter,
                        None => return None, // Should never get here
                    }
                }
            };

            if let Some((range, token)) = token_iter.next() {
                return Some((current_line.0, range, token));
            } else {
                // Go to next line
                self.current_line = self.line_iter.next();
                self.token_iter = None;
                return self.next();
            }
        }
    }

    pub struct RelativeTokenMapIterator<'a, T: Clone> {
        iter: TokenMapIterator<'a, T>,
        previous_token: Option<(&'a u32, &'a TokenRange, &'a T)>,
    }

    impl<'a, T: Clone> RelativeTokenMapIterator<'a, T> {
        fn new(token_map: &'a TokenMap<T>) -> Self {
            Self {
                iter: token_map.iter(),
                previous_token: None,
            }
        }
    }
    #[derive(Debug, Clone)]
    pub struct RelativeToken<'a, T: Clone> {
        pub delta_start: u32,
        pub delta_line: u32,
        pub length: u32,
        pub token: &'a T,
    }

    impl<'a, T: Clone> Iterator for RelativeTokenMapIterator<'a, T> {
        type Item = RelativeToken<'a, T>;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(previous_token) = &self.previous_token {
                if let Some(token) = self.iter.next() {
                    let delta_line = token.0 - previous_token.0;
                    // relative to 0 or the previous token’s start if they are on the same line
                    let delta_start = if delta_line == 0 {
                        token.1.start - previous_token.1.start
                    } else {
                        token.1.start
                    };

                    let relative_token = RelativeToken {
                        delta_line,
                        delta_start,
                        length: token.1.len(),
                        token: token.2,
                    };
                    self.previous_token = Some(token);
                    return Some(relative_token);
                } else {
                    return None;
                }
            } else {
                if let Some(token) = self.iter.next() {
                    let relative_token = RelativeToken {
                        delta_line: *token.0,
                        delta_start: token.1.start,
                        length: token.1.len(),
                        token: token.2,
                    };
                    self.previous_token = Some(token);
                    return Some(relative_token);
                } else {
                    return None;
                }
            }
        }
    }
}

pub type TokenMap<T> = token_map::TokenMap<T>;

/// Scan a string for an integer. This method handles regular integers
/// as well as integers encoded as hex, binary, or octal.
pub fn scan_integer(s: &str) -> Result<i64, ParseIntError> {
    let mut chars = s.chars().peekable();

    if s.len() > 2 && chars.nth(0).unwrap_or('\0') == '0' {
        match chars.peek() {
            Some('x') | Some('X') => return i64::from_str_radix(&s[2..], 16),
            Some('b') | Some('B') => return i64::from_str_radix(&s[2..], 2),
            Some('o') | Some('O') => return i64::from_str_radix(&s[2..], 8),
            _ => {}
        }
    }
    s.parse::<i64>()
}

/// Scan a string for a float.
pub fn scan_float(s: &str) -> Result<f64, ParseFloatError> {
    if s.eq_ignore_ascii_case("nan") {
        trace!("scan_float: {}", s);
        Ok(f64::NAN)
    } else {
        s.parse::<f64>()
    }
}

// Scan a string for a boolean.
pub fn scan_bool(s: &str) -> Result<bool, ()> {
    trace!("Scanning for boolean: {}", s);
    if s.eq_ignore_ascii_case("true") {
        Ok(true)
    } else if s.eq_ignore_ascii_case("false") {
        Ok(false)
    } else {
        Err(())
    }
}

#[cfg(test)]
mod test {

    use crate::lexers::{scan_bool, scan_float, scan_integer};
    use crate::lexers::{Location, TokenRange};
    use core::cmp::Ordering;

    use super::TokenMap;

    #[test]
    fn test_range() {
        assert_eq!(TokenRange::new(5, 4), None);
        assert_eq!(TokenRange::new(5, 5), None);

        if let Some(r1) = TokenRange::new(5, 6) {
            assert_eq!(r1.len(), 1)
        } else {
            assert!(false);
        }

        // Partial Intersection
        let r1 = TokenRange::new(5, 11).unwrap();
        assert_eq!(r1.len(), 6);
        let r2 = TokenRange::new(8, 14).unwrap();
        assert_eq!(r2.len(), 6);

        assert_eq!(r1.partial_cmp(&r2), None);
        assert_eq!(r2.partial_cmp(&r1), None);

        assert_eq!(r1.intersection(&r2), TokenRange::new(8, 11));
        assert_eq!(r2.intersection(&r1), TokenRange::new(8, 11));

        assert_eq!(r1.difference(&r1), vec![]);
        assert_eq!(r2.difference(&r2), vec![]);

        assert_eq!(r1.difference(&r2), vec![TokenRange::new(5, 8).unwrap()]);
        assert_eq!(r2.difference(&r1), vec![TokenRange::new(11, 14).unwrap()]);

        // Complete intersection
        let r1 = TokenRange::new(5, 11).unwrap();
        let r2 = TokenRange::new(3, 14).unwrap();

        assert_eq!(r1.intersection(&r1), Some(r1));
        assert_eq!(r2.intersection(&r2), Some(r2));

        assert_eq!(r1.overlaps(&r2), true);
        assert_eq!(r2.overlaps(&r1), true);

        assert_eq!(r1.partial_cmp(&r2), None);
        assert_eq!(r2.partial_cmp(&r1), None);

        assert_eq!(r1.intersection(&r2), TokenRange::new(5, 11));
        assert_eq!(r2.intersection(&r1), TokenRange::new(5, 11));

        assert_eq!(r1.difference(&r2), vec![]);
        assert_eq!(
            r2.difference(&r1),
            vec![
                TokenRange::new(3, 5).unwrap(),
                TokenRange::new(11, 14).unwrap()
            ]
        );

        // Overlap one character
        let r1 = TokenRange::new(5, 12).unwrap();
        let r2 = TokenRange::new(11, 14).unwrap();

        assert_eq!(r1.overlaps(&r2), true);
        assert_eq!(r2.overlaps(&r1), true);

        assert_eq!(r1.partial_cmp(&r2), None);
        assert_eq!(r2.partial_cmp(&r1), None);

        assert_eq!(r1.intersection(&r2), TokenRange::new(11, 12));
        assert_eq!(r2.intersection(&r1), TokenRange::new(11, 12));

        assert_eq!(r1.difference(&r2), vec![TokenRange::new(5, 11).unwrap()]);
        assert_eq!(r2.difference(&r1), vec![TokenRange::new(12, 14).unwrap()]);

        // Entirely before/after
        let r1 = TokenRange::new(5, 11).unwrap();
        let r2 = TokenRange::new(11, 14).unwrap();
        assert_eq!(r1.overlaps(&r2), false);
        assert_eq!(r2.overlaps(&r1), false);

        assert_eq!(r1.partial_cmp(&r2), Some(Ordering::Less));
        assert_eq!(r2.partial_cmp(&r1), Some(Ordering::Greater));

        assert_eq!(r1.intersection(&r2), None);
        assert_eq!(r2.intersection(&r1), None);

        assert_eq!(r1.difference(&r2), vec![r1.clone()]);
        assert_eq!(r2.difference(&r1), vec![r2.clone()]);
    }

    #[test]
    pub fn test_token_map() -> anyhow::Result<()> {
        use tracing::info;

        // use tracing::level_filters::LevelFilter;
        // use tracing_subscriber::fmt::writer::BoxMakeWriter;
        // use tracing_subscriber::prelude::*;
        // use tracing_subscriber::{fmt, EnvFilter, Registry};
        // let filter = EnvFilter::builder()
        //     .with_default_directive(LevelFilter::INFO.into())
        //     .from_env()?
        //     .add_directive("moos_parser=trace".parse()?);
        // let writer = BoxMakeWriter::new(std::io::stderr);
        // let fmt_layer = tracing_subscriber::fmt::layer()
        //     .with_writer(writer)
        //     .with_ansi(false)
        //     .with_filter(filter);

        // Registry::default().with(fmt_layer).try_init()?;

        let mut tokens: TokenMap<String> = TokenMap::new();
        // This ${FOO} is a Test ${BAR} comment
        //
        // ${BAZ} another comment
        let inserted = tokens.insert_location(
            Location::new(0, 8),
            Location::new(0, 14),
            "Variable(FOO)".to_string(),
        );
        assert!(inserted);
        let inserted = tokens.insert_location(
            Location::new(0, 25),
            Location::new(0, 31),
            "Variable(BAR)".to_string(),
        );
        assert!(inserted);
        let inserted = tokens.insert_location(
            Location::new(0, 0),
            Location::new(0, 39),
            "Comment".to_string(),
        );
        assert!(inserted);

        let inserted = tokens.insert_location(
            Location::new(2, 3),
            Location::new(2, 9),
            "Variable(BAZ)".to_string(),
        );
        assert!(inserted);

        let inserted = tokens.insert_location(
            Location::new(2, 0),
            Location::new(2, 9),
            "Comment".to_string(),
        );
        assert!(inserted);

        // Dummy insert - This should fail
        let inserted = tokens.insert_location(
            Location::new(0, 0),
            Location::new(0, 25),
            "Comment".to_string(),
        );
        assert!(inserted == false);

        if let Some(iter) = tokens.line_iter(2) {
            iter.for_each(|token| {
                info!("Line 2 {token:?}");
            });
        }

        tokens.iter().for_each(|token| {
            info!("Token: {token:?}");
        });

        tokens.relative_iter().for_each(|token| {
            info!("Relative Token: {token:?}");
        });

        Ok(())
    }

    #[test]
    fn test_scan_integer() {
        // Regular Integer
        assert_eq!(scan_integer("12345"), Ok(12345));
        // Another Integer
        assert_eq!(scan_integer("-12345"), Ok(-12345));

        // Hex Integer
        assert_eq!(scan_integer("0xffff"), Ok(65535));
        assert_eq!(scan_integer("0Xffff"), Ok(65535));
        assert_eq!(scan_integer("0xFFFF"), Ok(65535));
        assert_eq!(scan_integer("0XFFFF"), Ok(65535));

        // Binary Integer
        assert_eq!(scan_integer("0b11111111"), Ok(255));
        assert_eq!(scan_integer("0B11111111"), Ok(255));

        // Octal
        assert_eq!(scan_integer("0o10"), Ok(8));
        assert_eq!(scan_integer("0O10"), Ok(8));

        assert_eq!(scan_integer("102d"), "102d".parse::<i64>());
        assert!(scan_integer("102d").is_err());
    }

    #[test]
    fn test_scan_float() {
        let approx_eq = |lhs: f64, rhs: f64, delta: f64| -> bool {
            if lhs.is_finite() && rhs.is_finite() {
                (lhs - rhs).abs() <= delta
            } else if lhs.is_nan() && rhs.is_nan() {
                true
            } else {
                lhs == rhs
            }
        };
        assert!(approx_eq(scan_float("12341.0").unwrap(), 12341.0, 0.0001));
        assert!(approx_eq(scan_float("-12341.0").unwrap(), -12341.0, 0.0001));
        assert!(approx_eq(scan_float("2.23e3").unwrap(), 2230.0, 0.0001));

        assert!(approx_eq(
            scan_float("-inf").unwrap(),
            f64::NEG_INFINITY,
            0.0001
        ));
        assert!(approx_eq(scan_float("inf").unwrap(), f64::INFINITY, 0.0001));
        assert!(approx_eq(scan_float("nan").unwrap(), f64::NAN, 0.0001));
    }

    #[test]
    fn test_scan_bool() {
        assert_eq!(scan_bool("true"), Ok(true));
        assert_eq!(scan_bool("True"), Ok(true));
        assert_eq!(scan_bool("TRUE"), Ok(true));

        assert_eq!(scan_bool("false"), Ok(false));
        assert_eq!(scan_bool("False"), Ok(false));
        assert_eq!(scan_bool("FALSE"), Ok(false));
    }
}
