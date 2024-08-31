use super::error::BehaviorParseError;
use crate::lexers::{scan_bool, scan_float, scan_integer, Location};

use core::cmp::max;
use core::str;
use core::str::CharIndices;
use lalrpop_util::ErrorRecovery;
use std::collections::{HashMap, VecDeque};
use std::iter::{Chain, Repeat, Skip};
use tracing::trace;

pub type Spanned<Token, Loc, Error> = Result<(Loc, Token, Loc), Error>;
pub type TokenQueue<'input> = VecDeque<Spanned<Token<'input>, Location, BehaviorParseError>>;

const DEFERRED_INITIALIZE_KEYWORD: &str = "initialize_";
const INITIALIZE_KEYWORD: &str = "initialize";
const BEHAVIOR_BLOCK_KEYWORD: &str = "Behavior";

#[derive(Debug, Default, Clone)]
pub struct State<'input> {
    pub errors: Vec<ErrorRecovery<Location, Token<'input>, BehaviorParseError>>,
    pub initializations: HashMap<String, String>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Token<'input> {
    Comment(&'input str),
    QuoteBegin,
    QuoteEnd,
    Boolean(bool, &'input str),
    Integer(i64, &'input str),
    Float(f64, &'input str),
    ValueString(&'input str),
    EnvVariable(&'input str),
    PartialEnvVariable(&'input str),
    DeferredInitializeKeyword,
    InitializeKeyword,
    BehaviorBlockKeyword,
    AssignmentOp,
    CurlyOpen,
    CurlyClose,
    /// End of Line
    EOL,
    /// End of File
    EOF,
}

pub struct Lexer<'input> {
    iter: std::iter::Zip<
        CharIndices<'input>,
        Chain<Skip<CharIndices<'input>>, Repeat<(usize, char)>>,
    >,
    input: &'input str,
    previous_index: Option<usize>,
    line_number: u32,
    char_count: usize,
    start_of_line: bool,
    found_assign_op: bool,
    trim_start: bool,
    trim_end: bool,
    token_queue: TokenQueue<'input>,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        // Create a Zip iterator to allow looking that the current character
        // and the next character

        let iter = input.char_indices().zip(
            input
                .char_indices()
                .skip(1)
                .chain(std::iter::repeat((input.len(), '\0'))),
        );

        let previous_index = Some(0);
        Lexer {
            iter,
            input,
            previous_index,
            line_number: 0,
            char_count: 0,
            start_of_line: true,
            found_assign_op: false,
            trim_start: true,
            trim_end: false,
            token_queue: TokenQueue::new(),
        }
    }

    #[inline]
    pub(crate) fn get_location(&self, index: usize) -> Location {
        Location::new(
            self.line_number as u32,
            max(index - self.char_count, 0) as u32,
        )
    }

    #[inline]
    fn push_token(&mut self, start_index: usize, token: Token<'input>, end_index: usize) {
        self.token_queue.push_back(Ok((
            self.get_location(start_index),
            token,
            self.get_location(end_index),
        )));
    }

    /*
     * Closure for safely getting an index from `self.input`.
     */
    #[inline]
    fn get_safe_index(&self, i: usize) -> Option<usize> {
        if i < self.input.len() {
            Some(i)
        } else {
            None
        }
    }

    /**
     * Get the unhandled string from an the previous index to the current index
     * of a string.
     *
     * # Parameters:
     * * `index`: Current index
     * * `auto_trim`: Enable auto trim. This uses `self.trim_start` and
     * `self.trim_end` to handle trimming.
     */
    #[inline]
    fn get_unhandled_string(&self, index: usize, auto_trim: bool) -> Option<(usize, &'input str)> {
        if let Some(prev_i) = self.previous_index {
            let start_index = if auto_trim && self.trim_start {
                if let Some(index_after_whitespace) =
                    self.input[prev_i..index].find(|c| c != ' ' && c != '\t')
                {
                    prev_i + index_after_whitespace
                } else {
                    prev_i
                }
            } else {
                prev_i
            };

            let unhandled = &self.input[start_index..index];

            // If auto_trim is
            let unhandled = if auto_trim {
                if self.trim_start && self.trim_end {
                    unhandled.trim()
                } else if self.trim_start {
                    unhandled.trim_start()
                } else if self.trim_end {
                    unhandled.trim_end()
                } else {
                    unhandled
                }
            } else {
                unhandled
            };

            return if unhandled.is_empty() {
                None
            } else {
                Some((start_index, unhandled))
            };
        }
        None
    }

    #[inline]
    fn _handle_new_line(&mut self, i: usize) {
        self.token_queue
            .push_back(Ok((self.get_location(i), Token::EOL, self.get_location(i))));
        self.line_number += 1;
        self.char_count = i + 1;
        self.start_of_line = true;
        self.found_assign_op = false;
        self.trim_start = true;
        self.trim_end = false;
        self.previous_index = self.get_safe_index(i + 1);
    }

    #[inline]
    fn scan_keywords_and_values(&mut self, line: &'input str, line_index: usize) {
        // Sanity check that the input is at least 1 characters
        if line.len() < 1 {
            return;
        }

        let (input, line_index) = if self.start_of_line {
            let line_trim_start = line.trim_start();

            let keywords = [
                (INITIALIZE_KEYWORD, Token::InitializeKeyword),
                (
                    DEFERRED_INITIALIZE_KEYWORD,
                    Token::DeferredInitializeKeyword,
                ),
                (BEHAVIOR_BLOCK_KEYWORD, Token::BehaviorBlockKeyword),
            ];

            // Check for keywords
            if let Some(first_word) = line.split_whitespace().next() {
                trace!(
                    "First Word: {:?}, {} {}",
                    first_word,
                    line_index,
                    first_word.len()
                );

                let mut iter = keywords
                    .iter()
                    .filter(|keyword| keyword.0.eq_ignore_ascii_case(first_word));

                if let Some(keyword) = iter.next() {
                    // Handle block keywords
                    let new_line_index = line_index + first_word.len();
                    self.push_token(line_index, keyword.1, new_line_index);

                    (&line[first_word.len()..], new_line_index)
                } else {
                    (line, line_index)
                }
            } else {
                (line, line_index)
            }
        } else {
            (line, line_index)
        };

        // Start of variable not found
        if !input.is_empty() {
            self.scan_value(input, line_index);
        }
    }

    fn scan_value(&mut self, line: &'input str, line_index: usize) {
        if line.is_empty() {
            return;
        }
        self.trim_start = false;

        if self.found_assign_op {
            if let Ok(value) = scan_integer(line) {
                self.push_token(
                    line_index,
                    Token::Integer(value, line),
                    line_index + line.len(),
                );
                return;
            } else if let Ok(value) = scan_float(line) {
                self.push_token(
                    line_index,
                    Token::Float(value, line),
                    line_index + line.len(),
                );
                return;
            } else if let Ok(value) = scan_bool(line) {
                self.push_token(
                    line_index,
                    Token::Boolean(value, line),
                    line_index + line.len(),
                );
                return;
            }
        }

        self.push_token(
            line_index,
            Token::ValueString(&line),
            line_index + line.len(),
        );
    }

    fn tokenize_macro(&mut self, _i: usize) {
        // If its not the start of the line, it can't be a macro.
        if !self.start_of_line {
            return;
        }

        if let Some((_prev_i, unhandled)) = self.get_unhandled_string(self.input.len(), true) {
            if !unhandled.trim_start().starts_with("#") {
                return;
            }
        }
        // Skip lines that start with #

        // TODO: We should only skip lines that start with known macros

        while let Some(((i, c), (_ii, _cc))) = self.iter.find(|&((_i, c), (_ii, _cc))| c == '\n') {
            match c {
                '\n' => {
                    // Setting the previous index to drop previous tokens
                    self.previous_index = self.get_safe_index(i);
                    self.tokenize_new_line(i, false);
                    return;
                }
                _ => {}
            }
        }

        // Should only get in here if we have reached the end of the input.
        self.previous_index = self.get_safe_index(self.input.len());
    }

    fn tokenize_new_line(&mut self, i: usize, drop_unhandled: bool) {
        // Trim up to the new line
        self.trim_end = true;
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() && !drop_unhandled {
                self.scan_keywords_and_values(unhandled, prev_i);
            }
        }
        self._handle_new_line(i);
    }

    fn tokenize_assignment_op(&mut self, i: usize) {
        // Trim up to the assignment op
        self.trim_end = true;
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
            }
        }
        self.token_queue.push_back(Ok((
            self.get_location(i),
            Token::AssignmentOp,
            self.get_location(i),
        )));
        self.found_assign_op = true;
        self.start_of_line = false;
        self.trim_start = true;
        self.trim_end = false;
        self.previous_index = self.get_safe_index(i + 1);
    }

    fn tokenize_curly_brace(&mut self, i: usize, token: Token<'input>) {
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
            }
        }
        self.token_queue
            .push_back(Ok((self.get_location(i), token, self.get_location(i))));
        self.trim_start = true;
        self.trim_end = false;
        self.start_of_line = false;
        self.previous_index = self.get_safe_index(i + 1);
    }

    /// Tokenize a quote.
    /// Returns true if a full quote is found; false if the end of the line
    /// or end of the file is reached without finding the matching quote.
    fn tokenize_quote(&mut self, i: usize) -> bool {
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
                self.start_of_line = false;
                self.trim_start = false;
            }
        }

        self.push_token(i, Token::QuoteBegin, i + 1);
        self.previous_index = self.get_safe_index(i + 1);

        while let Some(((ii, cc), (_iii, ccc))) = self.iter.find(|&((_ii, cc), (_iii, ccc))| {
            cc == '"' || cc == '\n' || (cc == '$' && ccc == '(') || (cc == '%' && ccc == '(')
        }) {
            match cc {
                '"' => {
                    // Push any unhandled tokens before the QuoteEnd
                    if let Some((prev_i, unhandled)) = self.get_unhandled_string(ii, false) {
                        if !unhandled.is_empty() {
                            self.scan_keywords_and_values(unhandled, prev_i);
                        }
                    }
                    self.push_token(ii, Token::QuoteEnd, ii + 1);
                    self.trim_start = false;
                    self.previous_index = self.get_safe_index(ii + 1);
                    return true;
                }
                // Handle Variables inside of quotes
                cc if (cc == '$' && ccc == '{') => {
                    let found_variable = self.tokenize_variable(
                        ii,
                        cc,
                        ccc,
                        |text: &'input str, is_partial: bool| {
                            if !is_partial {
                                Token::EnvVariable(text)
                            } else {
                                Token::PartialEnvVariable(text)
                            }
                        },
                    );
                    if !found_variable {
                        return false;
                    }
                }
                '\n' => {
                    // Push any unhandled tokens before the End of line
                    if let Some((prev_i, unhandled)) = self.get_unhandled_string(ii, false) {
                        if !unhandled.is_empty() {
                            self.scan_keywords_and_values(unhandled, prev_i);
                        }
                    }

                    self._handle_new_line(ii);
                    return false;
                }
                _ => {}
            }
        }

        // Reached the end of the input
        // Push any unhandled tokens before the end of the file
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(self.input.len(), false) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
            }
        }
        self.previous_index = None;
        return false;
    }

    fn tokenize_comment(&mut self, i: usize) {
        // Trim up to the comment
        self.trim_end = true;
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
                self.start_of_line = false;
                self.trim_start = false;
            }
        }
        // Comment - Skip over the second slash
        let _r = self.iter.next();
        if let Some(((ii, _cc), (_iii, _ccc))) =
            self.iter.find(|&((_ii, cc), (_iii, _ccc))| cc == '\n')
        {
            self.push_token(i, Token::Comment(&self.input[i + 2..ii].trim()), ii);
            self.previous_index = self.get_safe_index(ii + 1);

            self._handle_new_line(ii);
        } else {
            // Reached the end of the input
            self.push_token(
                i,
                Token::Comment(&self.input[i + 2..].trim()),
                self.input.len(),
            );
            self.previous_index = None;
        }
    }

    /// Tokenize an Env variable
    /// Returns true if a full variable is parsed; false if the end of the line
    /// or end of the file is reached without finding the ending token.
    fn tokenize_variable<F: Fn(&'input str, bool) -> Token<'input>>(
        &mut self,
        i: usize,
        _c: char,
        _cc: char,
        create_variable_func: F,
    ) -> bool {
        // Check for unhandled strings
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
                self.start_of_line = false;
                self.trim_start = false;
            }
        }
        // Find the matching end brace or paren or end of line

        while let Some(((ii, cc), (_iii, _ccc))) = self
            .iter
            .find(|&((_ii, cc), (_iii, _ccc))| cc == '\n' || cc == '}')
        {
            if cc == '\n' {
                // Partial Variable
                let token = create_variable_func(&self.input[i + 2..ii], true);
                self.push_token(i, token, ii);
                self.previous_index = self.get_safe_index(ii);
                self._handle_new_line(ii);
                return false;
            } else {
                // Variable
                let token = create_variable_func(&self.input[i + 2..ii], false);
                self.push_token(i, token, ii + 1);
                self.previous_index = self.get_safe_index(ii + 1);
                self.start_of_line = false;
                self.trim_start = false;
                return true;
            }
        }

        // Reached the end of the input - Partial Variable
        let token = create_variable_func(&self.input[i + 2..], true);
        self.push_token(i, token, self.input.len());
        self.previous_index = None;
        return false;
    }

    #[inline]
    fn tokenize(&mut self) {
        // Tokenize until we find:
        //   1. End of line
        //   2. Comment
        //   3. Env variable
        //   4. Initialize Keyword
        //   5. Behavior Block Keyword
        //   6. Open/Close Curly brace
        //   7. Assignment
        //   8. # Macro => Skip entire line
        //
        // Ignore other tokens

        let mut found_new_line = false;
        while let Some(((i, c), (_ii, cc))) = self.iter.find(|&((_i, c), (_ii, cc))| {
            c == '\n'
                || (c == '/' && cc == '/') // Comment
                || (c == '$' && cc == '{') // Env variable
                || (c == '=' && !self.found_assign_op) // Assignment
                || (c == '"') // Quote
                || (c == '#') // Macro
                || ((c == '{' || c == '}') && self.start_of_line) // Open/Close curly
        }) {
            match c {
                '\n' => {
                    self.tokenize_new_line(i, false);
                    found_new_line = true;
                    // Break out of the tokenize for-loop after each line
                    break;
                }
                '/' => self.tokenize_comment(i),
                c if (c == '$' && cc == '{') => {
                    self.tokenize_variable(i, c, cc, |text: &'input str, is_partial: bool| {
                        if !is_partial {
                            Token::EnvVariable(text)
                        } else {
                            Token::PartialEnvVariable(text)
                        }
                    });
                }
                '=' => self.tokenize_assignment_op(i),
                '"' => {
                    if !self.tokenize_quote(i) {
                        break;
                    }
                }
                '#' => self.tokenize_macro(i),
                '{' => self.tokenize_curly_brace(i, Token::CurlyOpen),
                '}' => self.tokenize_curly_brace(i, Token::CurlyClose),
                _ => {}
            }
        }

        if !found_new_line {
            // Should only get in here if we have reached the end of the input.
            // If so, check that there isn't some straggling unhandled string.
            self.trim_end = true;
            if let Some((prev_i, unhandled)) = self.get_unhandled_string(self.input.len(), true) {
                if !unhandled.is_empty() {
                    self.scan_keywords_and_values(unhandled, prev_i);
                }
                self.previous_index = self.get_safe_index(self.input.len());
            }
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token<'input>, Location, BehaviorParseError>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.token_queue.pop_front() {
            return Some(token);
        }
        self.tokenize();

        if let Some(token) = self.token_queue.pop_front() {
            Some(token)
        } else {
            None
        }
    }
}
