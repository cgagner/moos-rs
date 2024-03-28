use crate::error::MoosParseError;
use crate::lexers::{scan_bool, scan_float, scan_integer, Location};

use core::cmp::max;
use core::str;
use core::str::{CharIndices, ParseBoolError};
use lalrpop_util::ErrorRecovery;
use std::collections::{HashMap, VecDeque};
use std::iter::{Chain, Repeat, Skip};
use std::num::{ParseFloatError, ParseIntError};
use tracing::{debug, error, info, trace, warn};

pub type Spanned<Token, Loc, Error> = Result<(Loc, Token, Loc), Error>;
pub type TokenQueue<'input> = VecDeque<Spanned<Token<'input>, Location, MoosParseError<'input>>>;

#[derive(Debug, Default, Clone)]
pub struct State<'input> {
    pub errors: Vec<ErrorRecovery<Location, Token<'input>, MoosParseError<'input>>>,
    pub defines: HashMap<String, String>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Token<'input> {
    Comment(&'input str),
    Quote(&'input str),
    PartialQuote(&'input str, char),
    Boolean(bool, &'input str),
    Integer(i64, &'input str),
    Float(f64, &'input str),
    ValueString(&'input str),
    PlugVariable(&'input str),
    PartialPlugVariable(&'input str),
    PlugUpperVariable(&'input str),
    PartialPlugUpperVariable(&'input str),
    MacroDefine,
    MacroInclude,
    MacroIfDef,
    MacroIfNotDef,
    MacroElseIfDef,
    MacroElse,
    MacroEndIf,
    UnknownMacro(&'input str),
    OrOperator,
    AndOperator,
    /// End of Line
    EOL,
    /// End of File
    EOF,
}

pub trait TokenListener {
    fn handle_token(&mut self, token: &Token, start_loc: &Location, end_loc: &Location);
}

pub struct Lexer<'input, 'listen> {
    token_listeners: Vec<&'listen mut dyn TokenListener>,
    iter: std::iter::Zip<
        CharIndices<'input>,
        Chain<Skip<CharIndices<'input>>, Repeat<(usize, char)>>,
    >,
    input: &'input str,
    previous_index: Option<usize>,
    line_number: usize,
    char_count: usize,
    start_of_line: bool,
    found_assign_op: bool,
    trim_start: bool,
    trim_end: bool,
    token_queue: TokenQueue<'input>,
}

impl<'input, 'listen> Lexer<'input, 'listen> {
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
            token_listeners: vec![],
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

    pub fn add_listener(&mut self, token_listener: &'listen mut dyn TokenListener) {
        self.token_listeners.push(token_listener);
    }

    pub fn clear_listeners(&mut self) {
        self.token_listeners.clear();
    }

    #[inline]
    pub(crate) fn get_location(&self, index: usize) -> Location {
        Location::new(self.line_number, max(index - self.char_count, 0))
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
                    self.input[prev_i..index].find(|c| c != ' ' && c != '\t' && c != '\r')
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
            trace!(
                "get_unhandled_string: '{}' - trim_start: {} - trim_end: {}",
                unhandled,
                self.trim_start,
                self.trim_end,
            );

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

    #[inline]
    fn scan_keywords_and_values(&mut self, line: &'input str, line_index: usize) {
        // Sanity check that the input is at least 1 characters
        if line.len() < 1 {
            return;
        }
        trace!(
            "Scanning for variables: {:?}, start_of_line: {:?}",
            line,
            self.start_of_line
        );

        // Start of variable not found
        if !line.is_empty() {
            trace!("Last string: '{:?}'", line);
            self.scan_value(line, line_index);
        }
    }

    fn tokenize_or_operator(&mut self, i: usize) {
        self.iter.next();
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
            }
            self.previous_index = self.get_safe_index(i + 2);
        }

        self.push_token(i, Token::OrOperator, i + 2);
    }

    fn tokenize_and_operator(&mut self, i: usize) {
        self.iter.next();
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
            }
            self.previous_index = self.get_safe_index(i + 2);
        }

        self.push_token(i, Token::AndOperator, i + 2);
    }

    fn get_macro_token(line: &'input str) -> Token<'input> {
        match line {
            "define" => Token::MacroDefine,
            "else" => Token::MacroElse,
            "elseifdef" => Token::MacroElseIfDef,
            "endif" => Token::MacroEndIf,
            "ifdef" => Token::MacroIfDef,
            "ifndef" => Token::MacroIfNotDef,
            "include" => Token::MacroInclude,
            _ => Token::UnknownMacro(line),
        }
    }

    fn tokenize_macro(&mut self, i: usize) {
        // If its not the start of the line, it can't be a macro.
        if !self.start_of_line {
            return;
        }

        if let Some((_prev_i, unhandled)) = self.get_unhandled_string(self.input.len(), true) {
            if !unhandled.trim_start().starts_with("#") {
                return;
            }
        }
        // [endif|else] - [comment]
        // include [quote|string|variable] [comment]
        // define [key|variable] [value|variable]
        // [ifdef|elseifdef] [condition] [|| &&] [condition]
        // ifndef [key|variable]
        // [condition] => [string|quote|variable]

        // Find the macro by looking for the next whitespace, newline, or comment
        let (token, next_index) = if let Some(((ii, cc), (_iii, _ccc))) =
            self.iter.find(|&((_ii, cc), (_iii, ccc))| {
                cc == ' ' || cc == '\t'  || cc == '\r'  // Whitespace
                || cc == '\n'  // Newline
                || (cc == '/' && ccc == '/') // Comment
            }) {
            // Get the line
            let line = &self.input[i + 1..ii];
            let token = Self::get_macro_token(line);
            self.push_token(i, token, ii);
            self.previous_index = self.get_safe_index(ii);
            match cc {
                '\n' => {
                    self._handle_new_line(ii);
                    return;
                }
                '/' => {
                    self.tokenize_comment(ii);
                    return;
                }
                _ => {}
            }
            (token, ii)
        } else {
            // If we get here, we reached the end of the file.
            let line = &self.input[i + 1..];
            let token = Self::get_macro_token(line);
            self.push_token(i, token, self.input.len());
            self.previous_index = None;
            return;
        };

        let has_conditions = match token {
            Token::MacroIfDef | Token::MacroElseIfDef => true,
            _ => false,
        };

        let mut has_whitespace = match token {
            Token::MacroDefine | Token::MacroIfDef | Token::MacroElseIfDef => true,
            _ => false,
        };

        while let Some(((i, c), (_ii, cc))) = self.iter.find(|&((_i, c), (_ii, cc))| {
            c == '\n'
                || c == '"'
                || (c == '=')
                || (has_whitespace && (c == ' ' || c == '\t' || c == '\r')) // Whitespace
                || (c == '/' && cc == '/') // Comment
                || (c == '$' && (cc == '{' || cc == '(')) // Env or Plug variable
                || (c == '%' && cc == '(') // Plug Upper Variable
                || (has_conditions && c == '|' && cc == '|') // Or operator
                || (has_conditions && c == '&' && cc == '&' ) // And operator
        }) {
            match c {
                '\n' => {
                    self.tokenize_new_line(i);
                    return;
                }
                '/' => {
                    self.tokenize_comment(i);
                    return;
                }
                '"' => self.tokenize_quote(i),
                c if (c == '$' && (cc == '{' || cc == '(')) => self.tokenize_variable(i, c, cc),
                c if (c == '%' && cc == '(') => self.tokenize_upper_variable(i),
                '|' => {
                    self.trim_end = true;
                    self.tokenize_or_operator(i);
                    has_whitespace = true;
                    self.trim_start = true;
                    self.found_assign_op = false;
                }
                '&' => {
                    self.trim_end = true;
                    self.tokenize_and_operator(i);
                    has_whitespace = true;
                    self.trim_start = true;
                    self.found_assign_op = false;
                }
                ' ' | '\t' | '\r' => {
                    self.found_assign_op = true; // Enables parsing primitives
                    self.trim_end = true;
                    if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
                        if !unhandled.is_empty() {
                            self.scan_keywords_and_values(unhandled, prev_i);
                            has_whitespace = false;
                        }
                        self.previous_index = self.get_safe_index(i + 1);
                    }
                    self.trim_start = true;
                }
                _ => {}
            }
        }

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

    fn tokenize_new_line(&mut self, i: usize) {
        // Trim up to the new line
        self.trim_end = true;
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
            }
        }
        self._handle_new_line(i);
        // Break out of the tokenize for-loop after each line
    }

    fn tokenize_quote(&mut self, i: usize) {
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
                self.start_of_line = false;
                self.trim_start = false;
            }
        }
        let mut comment_index: Option<usize> = None;
        // Find the matching quote mark
        while let Some(((ii, cc), (_iii, _ccc))) = self
            .iter
            .find(|&((_ii, cc), (_iii, ccc))| cc == '"' || cc == '\n' || (cc == '/' && ccc == '/'))
        {
            match cc {
                '"' => {
                    self.push_token(i, Token::Quote(&self.input[i + 1..ii]), ii + 1);
                    self.trim_start = false;
                    self.previous_index = self.get_safe_index(ii + 1);
                    trace!("Found quote: {}", &self.input[i + 1..ii]);
                    return;
                }
                '\n' => {
                    if let Some(comment_index) = comment_index {
                        self.push_token(
                            i,
                            Token::PartialQuote(&self.input[i + 1..comment_index], '"'),
                            comment_index,
                        );
                        self.push_token(
                            comment_index,
                            Token::Comment(&self.input[comment_index + 2..ii].trim()),
                            ii,
                        );
                    } else {
                        self.push_token(i, Token::PartialQuote(&self.input[i + 1..ii], '"'), ii);
                    }
                    self._handle_new_line(ii);
                    return;
                }
                '/' => {
                    // TODO: Need to handle partial quotes and comments
                    comment_index = Some(ii);
                }
                _ => {}
            }
        }

        // Reached the end of the input

        if let Some(comment_index) = comment_index {
            self.push_token(
                i,
                Token::PartialQuote(&self.input[i + 1..comment_index], '"'),
                comment_index,
            );
            self.push_token(
                comment_index,
                Token::Comment(&self.input[comment_index + 2..].trim()),
                self.input.len(),
            );
        } else {
            self.push_token(
                i,
                Token::PartialQuote(&self.input[i + 1..], '"'),
                self.input.len(),
            );
        }

        self.previous_index = None;
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

    fn tokenize_variable(&mut self, i: usize, _c: char, cc: char) {
        // TODO: Should this handle quotes inside a variable?
        // E.G. ${"Test//Comment"}

        // Check for unhandled strings
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
                self.start_of_line = false;
                self.trim_start = false;
            }
        }
        // Find the matching end brace or paren or end of line

        let mut found_quote = false;
        while let Some(((ii, cc), (_iii, _ccc))) = self.iter.find(|&((_ii, cc), (_iii, ccc))| {
            cc == '\n' || cc == ')' || cc == '"' || (!found_quote && cc == '/' && ccc == '/')
        }) {
            if cc == '\n' || cc == '/' {
                // Partial Variable
                let token = Token::PartialPlugVariable(&self.input[i + 2..ii]);
                self.push_token(i, token, ii);
                self.previous_index = self.get_safe_index(ii);
                if cc == '/' {
                    self.tokenize_comment(ii);
                } else {
                    self._handle_new_line(ii);
                }
                return;
            } else if cc == '"' {
                found_quote = !found_quote;
            } else {
                // Variable
                let token = Token::PlugVariable(&self.input[i + 2..ii]);
                self.push_token(i, token, ii + 1);
                self.previous_index = self.get_safe_index(ii + 1);
                self.start_of_line = false;
                self.trim_start = false;
                return;
            }
        }

        // Reached the end of the input - Partial Variable
        let token = Token::PartialPlugVariable(&self.input[i + 2..]);
        self.push_token(i, token, self.input.len());
        self.previous_index = None;
    }

    fn tokenize_upper_variable(&mut self, i: usize) {
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
                self.start_of_line = false;
                self.trim_start = false;
            }
        }
        // Find the ending paren
        if let Some(((ii, cc), (_iii, _ccc))) = self
            .iter
            .find(|&((_ii, cc), (_iii, ccc))| cc == '\n' || cc == ')' || (cc == '/' && ccc == '/'))
        {
            if cc == '\n' || cc == '/' {
                // Partial Variable
                let token = Token::PartialPlugUpperVariable(&self.input[i + 2..ii]);
                self.push_token(i, token, ii);
                self.previous_index = self.get_safe_index(ii);
                if cc == '/' {
                    self.tokenize_comment(ii);
                } else {
                    self._handle_new_line(ii);
                }
            } else {
                // Variable
                let token = Token::PlugUpperVariable(&self.input[i + 2..ii]);
                self.push_token(i, token, ii + 1);
                self.previous_index = self.get_safe_index(ii + 1);
                self.start_of_line = false;
                self.trim_start = false;
            }
        } else {
            // Reached the end of the input - Partial Variable
            let token = Token::PartialPlugUpperVariable(&self.input[i + 2..]);
            self.push_token(i, token, self.input.len());
            self.previous_index = None;
        }
    }

    #[inline]
    fn tokenize(&mut self) {
        // Tokenize until we find:
        //   1. End of line
        //   2. Comment // TODO: Should we even scan for comments.
        //   3. Plug variable
        //   4. Plug upper variable
        //   5. Macro
        //
        // Ignore other tokens

        while let Some(((i, c), (_ii, cc))) = self.iter.find(|&((_i, c), (_ii, cc))| {
            c == '\n'
                || (c == '/' && cc == '/') // Comment
                || (c == '$' && cc == '(') // Plug variable
                || (c == '%' && cc == '(') // Plug Upper Variable
                || (c == '#') // Macro
        }) {
            match c {
                '\n' => {
                    self.tokenize_new_line(i);
                    // Break out of the tokenize for-loop after each line
                    break;
                }
                '/' => self.tokenize_comment(i),
                c if (c == '$' && (cc == '{' || cc == '(')) => self.tokenize_variable(i, c, cc),
                c if (c == '%' && cc == '(') => self.tokenize_upper_variable(i),
                '#' => self.tokenize_macro(i),
                _ => {}
            }
        }

        // NOTE: There could still be tokens to be parse, but we don't care
        // about them.
    }

    fn _next(&mut self) -> Option<Spanned<Token<'input>, Location, MoosParseError<'input>>> {
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

impl<'input, 'listen> Iterator for Lexer<'input, 'listen> {
    type Item = Spanned<Token<'input>, Location, MoosParseError<'input>>;
    fn next(&mut self) -> Option<Self::Item> {
        let rtn = self._next();

        for listener in &mut self.token_listeners {
            if let Some(Ok((start_loc, token, end_loc))) = rtn {
                listener.handle_token(&token, &start_loc, &end_loc);
            }
        }
        return rtn;
    }
}
