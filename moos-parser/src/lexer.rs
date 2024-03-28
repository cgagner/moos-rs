use crate::error::MoosParseError;

use crate::lexers::{scan_bool, scan_float, scan_integer, Location};
use core::cmp::max;
use core::str;
use core::str::{CharIndices, ParseBoolError};
use lalrpop_util::ErrorRecovery;
use std::collections::{HashMap, VecDeque};
use std::iter::{Chain, Repeat, Skip};
use std::num::{ParseFloatError, ParseIntError};
use tracing::{debug, trace};

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
    Key(&'input str),
    Boolean(bool, &'input str),
    Integer(i64, &'input str),
    Float(f64, &'input str),
    AssignOp,
    ParenOpen,
    ParenClose,
    CurlyOpen,
    CurlyClose,
    DefineKeyword,
    BlockKeyword(&'input str),
    ValueString(&'input str),
    EnvVariable(&'input str),
    PartialEnvVariable(&'input str),
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
    Whitespace(&'input str),
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
    block_keywords: Vec<&'static str>,
    keywords: Vec<&'static str>,
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
            block_keywords: vec!["processconfig", "behavior"], // TODO: This should only be one or the other
            keywords: vec!["define:", "set", "initialize"], // TODO: The keywords are based on the type of file
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

        let (input, line_index) = if self.start_of_line {
            let line_trim_start = line.trim_start();
            if line_trim_start.starts_with("define:") {
                let index = line
                    .find("define:")
                    .expect("Could not fine index of 'define:'");

                let index_after_define = index + "define:".len();
                let end_index = line_index + index_after_define;
                self.push_token(line_index + index, Token::DefineKeyword, end_index);
                if let Some(i) =
                    line[index_after_define..].find(|x| x != '\t' && x != ' ' && x != '\r')
                {
                    (&line[index_after_define + i..], end_index + i)
                } else {
                    // Nothing else to process
                    return;
                }
            } else {
                // Check for keywords
                if let Some(first_word) = line.split_whitespace().next() {
                    let first_word_lower = first_word.to_lowercase();
                    trace!(
                        "First Word: {:?}, {} {}",
                        first_word,
                        line_index,
                        first_word.len()
                    );
                    if self.block_keywords.contains(&first_word_lower.as_str()) {
                        // Handle block keywords
                        let new_line_index = line_index + first_word.len();
                        self.push_token(
                            line_index,
                            Token::BlockKeyword(first_word),
                            new_line_index,
                        );

                        (&line[first_word.len()..], new_line_index)
                    } else if self.keywords.contains(&first_word_lower.as_str()) {
                        // TODO: How do we know have a keyword token
                        (line, line_index)
                    } else {
                        (line, line_index)
                    }
                } else {
                    (line, line_index)
                }
            }
        } else {
            (line, line_index)
        };

        // Start of variable not found
        if !input.is_empty() {
            trace!("Last string: '{:?}'", input);
            self.scan_value(input, line_index);
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

        // // Find the end of the line or a comment
        // if let Some(((ii, cc), (_iii, _ccc))) = self
        //     .iter
        //     .find(|&((_ii, cc), (_iii, ccc))| cc == '\n' || (cc == '/' && ccc == '/'))
        // {
        //     // Get the line
        //     let line = &self.input[i..ii];
        //     self.scan_macro_str(i, &line);
        //     self.previous_index = self.get_safe_index(ii);
        //     match cc {
        //         '\n' => {
        //             self._handle_new_line(ii);
        //         }
        //         '/' => {
        //             self.tokenize_comment(ii);
        //         }
        //         _ => {}
        //     }
        // } else {
        //     // If we get here, we reached the end of the file.
        //     let line = &self.input[i..];
        //     self.scan_macro_str(i, &line);
        //     self.previous_index = None;
        // }

        // [endif|else] - [comment]
        // include [quote|string|variable] [comment]
        // define [key|variable] [value|variable] [comment]
        // [ifdef|elseifdef] [condition] [|| &&] [condition]
        // ifndef [key|variable] [comment]
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
                '=' => self.tokenize_assignment(i),
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

    fn tokenize_assignment(&mut self, i: usize) {
        if !self.found_assign_op {
            // Trim up to the assignment operator
            self.trim_end = true;
            if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
                if !unhandled.is_empty() {
                    self.scan_keywords_and_values(unhandled, prev_i);
                }
            }
            self.push_token(i, Token::AssignOp, i + 1);
            self.found_assign_op = true;
            self.start_of_line = false;
            self.trim_start = true;
            self.trim_end = false;
            self.previous_index = self.get_safe_index(i + 1);
        }
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
                    log::trace!("Found quote: {}", &self.input[i + 1..ii]);
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

    fn tokenize_paren(&mut self, i: usize, c: char) {
        let token = if c == '{' {
            Token::CurlyOpen
        } else {
            Token::CurlyClose
        };

        // Check for unhandled strings
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_keywords_and_values(unhandled, prev_i);
                self.start_of_line = false;
            }
        }

        self.push_token(i, token, i + 1); // Checked
        self.previous_index = self.get_safe_index(i + 1);
        self.start_of_line = false;
        // Preserve spaces
        self.trim_start = false;
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
        let ending = if cc == '{' { '}' } else { ')' };
        let mut found_quote = false;
        while let Some(((ii, cc), (_iii, _ccc))) = self.iter.find(|&((_ii, cc), (_iii, ccc))| {
            cc == '\n' || cc == ending || cc == '"' || (!found_quote && cc == '/' && ccc == '/')
        }) {
            if cc == '\n' || cc == '/' {
                // Partial Variable
                let token = if ending == '}' {
                    Token::PartialEnvVariable(&self.input[i + 2..ii])
                } else {
                    Token::PartialPlugVariable(&self.input[i + 2..ii])
                };
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
                let token = if ending == '}' {
                    Token::EnvVariable(&self.input[i + 2..ii])
                } else {
                    Token::PlugVariable(&self.input[i + 2..ii])
                };
                self.push_token(i, token, ii + 1);
                self.previous_index = self.get_safe_index(ii + 1);
                self.start_of_line = false;
                self.trim_start = false;
                return;
            }
        }

        // Reached the end of the input - Partial Variable
        let token = if ending == '}' {
            Token::PartialEnvVariable(&self.input[i + 2..])
        } else {
            Token::PartialPlugVariable(&self.input[i + 2..])
        };
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
        // MOOS configuration files are line based files. There are only a
        // handful of lines that can be used.
        //
        // `ProcessConfig = AppName` - Process config line
        // `{` Beginning of the process config block line
        // `}` End of the process config block line
        // `<name> = <value>` - Assignment
        // `define: name = value` - Local variable assignment
        //
        // Comment are stripped off of the line before checking for the above
        // line types. The config parser also checks that a comment is not
        // inside of quotes. E.G. `name = " test // test"`
        // All other lines are ignored by the Config reader.
        // Some applications may still use the file parser, but that
        // is not considered here.

        let mut found_new_line = false;
        while let Some(((i, c), (_ii, cc))) = self.iter.find(|&((_i, c), (_ii, cc))| {
            c == '\n'
                || c == '"'
                || (c == '=' && !self.found_assign_op)
                || ((c == '{' || c == '}'))
                || (c == '/' && cc == '/') // Comment
                || (c == '$' && (cc == '{' || cc == '(')) // Env or Plug variable
                || (c == '%' && cc == '(') // Plug Upper Variable
                || (c == '#') // Macro
        }) {
            match c {
                '\n' => {
                    self.tokenize_new_line(i);
                    found_new_line = true;
                    // Break out of the tokenize for-loop after each line
                    break;
                }
                '=' => self.tokenize_assignment(i),
                '"' => self.tokenize_quote(i),
                '/' => self.tokenize_comment(i),
                c if (c == '{' || c == '}') => self.tokenize_paren(i, c),
                c if (c == '$' && (cc == '{' || cc == '(')) => self.tokenize_variable(i, c, cc),
                c if (c == '%' && cc == '(') => self.tokenize_upper_variable(i),
                '#' => self.tokenize_macro(i),
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
// ----------------------------------------------------------------------------
// Tests
#[cfg(test)]
mod tests {
    use crate::{
        error::MoosParseError,
        lexer::{Lexer, Location, State, Token, TokenListener},
    };
    use tracing::{debug, trace};

    #[test]
    pub fn test_tokenize_variable() {
        let input = r#"${"Test//Comment"}"#;
        let mut lexer = Lexer::new(input);

        let expected_tokens = vec![Token::EnvVariable("\"Test//Comment\"")];
        check_tokens(&mut lexer, expected_tokens);
    }

    #[test]
    pub fn test_tokenize_full() {
        let input = r#"
        define: CONFIG_VAR = MyConfigVar
        "This is a quote"
        // This is a comment
        name = value
        a = TRUE
        b = 1
        name with space = value with space
        vehicle_name = ${HOST}-%(VEHICLE_NAME)_$(VEHICLE_ID)12345
        ${CONFIG_VAR} = ${OTHER_VAR}
        ${TEST//Comment} = Still a Comment
        ProcessConfig = MyApp 
        {   

        }
        
        ProcessConfig = MyApp2 
        {
            
        }"#;
        let mut lexer = Lexer::new(input);

        let expected_tokens = vec![
            Token::EOL,
            Token::DefineKeyword,
            Token::ValueString("CONFIG_VAR"),
            Token::AssignOp,
            Token::ValueString("MyConfigVar"),
            Token::EOL,
            Token::Quote("This is a quote"),
            Token::EOL,
            Token::Comment("This is a comment"),
            Token::EOL,
            Token::ValueString("name"),
            Token::AssignOp,
            Token::ValueString("value"),
            Token::EOL,
            Token::ValueString("a"),
            Token::AssignOp,
            Token::Boolean(true, "TRUE"),
            Token::EOL,
            Token::ValueString("b"),
            Token::AssignOp,
            Token::Integer(1, "1"),
            Token::EOL,
            Token::ValueString("name with space"),
            Token::AssignOp,
            Token::ValueString("value with space"),
            Token::EOL,
            Token::ValueString("vehicle_name"),
            Token::AssignOp,
            Token::EnvVariable("HOST"),
            Token::ValueString("-"),
            Token::PlugUpperVariable("VEHICLE_NAME"),
            Token::ValueString("_"),
            Token::PlugVariable("VEHICLE_ID"),
            Token::Integer(12345, "12345"),
            Token::EOL,
            Token::EnvVariable("CONFIG_VAR"),
            Token::AssignOp,
            Token::EnvVariable("OTHER_VAR"),
            Token::EOL,
            Token::PartialEnvVariable("TEST"),
            Token::Comment("Comment} = Still a Comment"),
            Token::EOL,
            Token::BlockKeyword("ProcessConfig"),
            Token::AssignOp,
            Token::ValueString("MyApp"),
            Token::EOL,
            Token::CurlyOpen,
            Token::EOL,
            Token::EOL,
            Token::CurlyClose,
            Token::EOL,
            Token::EOL,
            Token::BlockKeyword("ProcessConfig"),
            Token::AssignOp,
            Token::ValueString("MyApp2"),
            Token::EOL,
            Token::CurlyOpen,
            Token::EOL,
            Token::EOL,
            Token::CurlyClose,
        ];
        check_tokens(&mut lexer, expected_tokens);
        trace!("Finihsed checking tokens....");
        for t in lexer {
            trace!("Token: {:?}", t);
        }
    }

    #[test]
    pub fn test_scan_variable2() {
        let input = "${MY_VAR}";
        let mut lexer = Lexer::new(input);

        let expected_tokens = vec![Token::EnvVariable("MY_VAR")];

        check_tokens(&mut lexer, expected_tokens);

        let input = "${MY_VAR}\n";
        let mut lexer = Lexer::new(input);

        let expected_tokens = vec![Token::EnvVariable("MY_VAR"), Token::EOL];

        check_tokens(&mut lexer, expected_tokens);

        let input = "${THIS_is_a_VARIABLE}\n";
        let mut lexer = Lexer::new(input);
        let iter = lexer.next();
        assert_eq!(
            (
                Location::new(0, 0),
                Token::EnvVariable("THIS_is_a_VARIABLE"),
                Location::new(0, 21)
            ),
            iter.unwrap().unwrap()
        );
        let iter = lexer.next();
        assert_eq!(
            (
                Location::new(0, (input.len() - 1) as u32),
                Token::EOL,
                Location::new(0, (input.len() - 1) as u32)
            ),
            iter.unwrap().unwrap()
        );
        // Test Partial Variables
        let input = "${MY_VAR";
        let mut lexer = Lexer::new(input);

        let expected_tokens = vec![Token::PartialEnvVariable("MY_VAR")];

        check_tokens(&mut lexer, expected_tokens);

        let input = "${MY_VAR\n";
        let mut lexer = Lexer::new(input);

        let expected_tokens = vec![Token::PartialEnvVariable("MY_VAR"), Token::EOL];

        check_tokens(&mut lexer, expected_tokens);

        let input = "${THIS_is_a_VARIABLE\n";
        let mut lexer = Lexer::new(input);
        let iter = lexer.next();
        assert_eq!(
            (
                Location::new(0, 0),
                Token::PartialEnvVariable("THIS_is_a_VARIABLE"),
                Location::new(0, 20)
            ),
            iter.unwrap().unwrap()
        );
        let iter = lexer.next();
        assert_eq!(
            (
                Location::new(0, (input.len() - 1) as u32),
                Token::EOL,
                Location::new(0, (input.len() - 1) as u32),
            ),
            iter.unwrap().unwrap()
        );
    }

    #[test]
    pub fn test_scan_comment() {
        let input = r#"  // This is a "comment""#;
        let mut lexer = Lexer::new(input);
        let iter = lexer.next();
        assert_eq!(
            (
                Location::new(0, 2),
                Token::Comment("This is a \"comment\""),
                Location::new(0, 24),
            ),
            iter.unwrap().unwrap()
        );

        let input = r#" ProcessConfig = MyApp // This is a "comment""#;
        let mut lexer = Lexer::new(input);

        let expected_tokens = vec![
            Token::BlockKeyword("ProcessConfig"),
            Token::AssignOp,
            Token::ValueString("MyApp"),
            Token::Comment("This is a \"comment\""),
        ];
        check_tokens(&mut lexer, expected_tokens);

        // TODO: Currently values allow multi-line strings if you end the line
        // with a backslash to escape the new line

        let input = r#"
        name1 = value1 // This is a "comment"
        name2 = value2 // Second Comment
        name3 = this\
        is \
        a test\
        of a multi-line string"#;
        let mut lexer = Lexer::new(input);

        let expected_tokens = vec![
            Token::EOL,
            Token::ValueString("name1"),
            Token::AssignOp,
            Token::ValueString("value1"),
            Token::Comment("This is a \"comment\""),
            Token::EOL,
            Token::ValueString("name2"),
            Token::AssignOp,
            Token::ValueString("value2"),
            Token::Comment("Second Comment"),
        ];
        check_tokens(&mut lexer, expected_tokens);

        for t in lexer {
            trace!("Token: {:?}", t);
        }
    }

    #[test]
    pub fn test_scan_value1() {
        let input = r#"TestValue = This is a Test "Comment // Test" \"// Actual Comment"#;
        // MOOS will return this: 'This is a Test Comment // Test \"'
        let mut lexer = Lexer::new(input);
        let expected_tokens = vec![
            Token::ValueString("TestValue"),
            Token::AssignOp,
            Token::ValueString("This is a Test "),
            Token::Quote("Comment // Test"),
            Token::ValueString(" \\"),
            Token::PartialQuote("", '"'),
            Token::Comment("Actual Comment"),
        ];
        check_tokens(&mut lexer, expected_tokens);
    }
    #[test]
    pub fn test_scan_value2() {
        let input = r#"TestValue = This is a Test  \"// Actual Comment"#;
        // MOOS will return this: 'This is a Test Comment // Test \"'
        let mut lexer = Lexer::new(input);
        let expected_tokens = vec![
            Token::ValueString("TestValue"),
            Token::AssignOp,
            Token::ValueString("This is a Test  \\"),
            Token::PartialQuote("", '"'),
            Token::Comment("Actual Comment"),
        ];
        check_tokens(&mut lexer, expected_tokens);
    }

    fn check_tokens(lexer: &mut Lexer, expected_tokens: Vec<Token>) {
        trace!("check_tokens!!!!!!!!!!!!!!!!!!!!!!!!!!");
        let mut i = 0;
        while let Some(Ok((_, token, _))) = lexer.next() {
            trace!("Token: {:?}", token);
            if i < expected_tokens.len() {
                assert_eq!(token, expected_tokens[i]);
                i += 1;
            } else {
                break;
            }
        }
        assert_eq!(i, expected_tokens.len());
    }

    #[test]
    fn test_primitives() {
        let input = r#"
        // This is a test float
        a = 12345.0
        b = 12345

        // Another Float
        c = -12341.0
        d = -12341

        // Scientific Notation
        e = 2.23e3
        f = +1.0
        g = -inf
        h = true
        i = False
        j = TRUE
        k = trues
        l = "true"
        m = "FALSE"
        "#;

        // NOTE: This uses to check single quote marks. However, I don't think
        // those are supported in MOOS

        let mut lexer = Lexer::new(input);

        let expected_tokens = vec![
            Token::EOL,
            Token::Comment("This is a test float"),
            Token::EOL,
            Token::ValueString("a"),
            Token::AssignOp,
            Token::Float(12345.0, "12345.0"),
            Token::EOL,
            Token::ValueString("b"),
            Token::AssignOp,
            Token::Integer(12345, "12345"),
            Token::EOL,
            Token::EOL,
            Token::Comment("Another Float"),
            Token::EOL,
            Token::ValueString("c"),
            Token::AssignOp,
            Token::Float(-12341.0, "-12341.0"),
            Token::EOL,
            Token::ValueString("d"),
            Token::AssignOp,
            Token::Integer(-12341, "-12341"),
            Token::EOL,
            Token::EOL,
            Token::Comment("Scientific Notation"),
            Token::EOL,
            Token::ValueString("e"),
            Token::AssignOp,
            Token::Float(2230.0, "2.23e3"),
            Token::EOL,
            Token::ValueString("f"),
            Token::AssignOp,
            Token::Float(1.0, "+1.0"),
            Token::EOL,
            Token::ValueString("g"),
            Token::AssignOp,
            Token::Float(f64::NEG_INFINITY, "-inf"),
            Token::EOL,
            Token::ValueString("h"),
            Token::AssignOp,
            Token::Boolean(true, "true"),
            Token::EOL,
            Token::ValueString("i"),
            Token::AssignOp,
            Token::Boolean(false, "False"),
            Token::EOL,
            Token::ValueString("j"),
            Token::AssignOp,
            Token::Boolean(true, "TRUE"),
            Token::EOL,
            Token::ValueString("k"),
            Token::AssignOp,
            Token::ValueString("trues"),
            Token::EOL,
            Token::ValueString("l"),
            Token::AssignOp,
            Token::Quote("true"),
            Token::EOL,
            Token::ValueString("m"),
            Token::AssignOp,
            Token::Quote("FALSE"),
            Token::EOL,
        ];
        check_tokens(&mut lexer, expected_tokens);
    }

    #[test]
    fn test_scan_macro() {
        let input = r#"
        #include asdf 
        #include "Test.plug"
        #define VALUE00  This is a test
        #define VALUE01 This is a test // Comment after define

        #ifdef VALUE1 12 // Test Comment  
        #else // Comment
        #define VALUE2 "this is a quote"
        #include "filepath.txt"
        #else // Comment
        #ifdef VALUE3 12 || VALUE4 123
        #endif // Comments
        #ifdef VALUE3 12 is a number && VALUE4 123
        #endfi // Unknown macro
        "#;

        let mut lexer = Lexer::new(input);
        while let Some(Ok((_, token, _))) = lexer.next() {
            trace!("test_scan_macro Token: {:?}", token);
        }

        let mut lexer = Lexer::new(input);
        let expected_tokens = vec![
            Token::EOL,
            Token::MacroInclude,
            Token::ValueString("asdf"),
            Token::EOL,
            Token::MacroInclude,
            Token::Quote("Test.plug"),
            Token::EOL,
            Token::MacroDefine,
            Token::ValueString("VALUE00"),
            Token::ValueString("This is a test"),
            Token::EOL,
            Token::MacroDefine,
            Token::ValueString("VALUE01"),
            Token::ValueString("This is a test"),
            Token::Comment("Comment after define"),
            Token::EOL,
            Token::EOL,
            Token::MacroIfDef,
            Token::ValueString("VALUE1"),
            Token::Integer(12, "12"),
            Token::Comment("Test Comment"),
            Token::EOL,
            Token::MacroElse,
            Token::Comment("Comment"),
            Token::EOL,
            Token::MacroDefine,
            Token::ValueString("VALUE2"),
            Token::Quote("this is a quote"),
            Token::EOL,
            Token::MacroInclude,
            Token::Quote("filepath.txt"),
            Token::EOL,
            Token::MacroElse,
            Token::Comment("Comment"),
            Token::EOL,
            Token::MacroIfDef,
            Token::ValueString("VALUE3"),
            Token::Integer(12, "12"),
            Token::OrOperator,
            Token::ValueString("VALUE4"),
            Token::Integer(123, "123"),
            Token::EOL,
            Token::MacroEndIf,
            Token::Comment("Comments"),
            Token::EOL,
            Token::MacroIfDef,
            Token::ValueString("VALUE3"),
            Token::ValueString("12 is a number"),
            Token::AndOperator,
            Token::ValueString("VALUE4"),
            Token::Integer(123, "123"),
            Token::EOL,
            Token::UnknownMacro("endfi"),
            Token::Comment("Unknown macro"),
            Token::EOL,
            // TODO: Need to finish added test cases for macros
        ];
        check_tokens(&mut lexer, expected_tokens);
    }

    #[test]
    fn test_scan_keywords_and_values() {
        // TODO: Does this need to account for nested parens? $(VAL(123))
        let input =
            "targ_%(VEHICLE_NAME)_test1_${VEHICLE_ID}_test2_$(DATE).moos${END}${TEST//Comment}";
        let input = "targ_%(VEHICLE_NAME)_test1_${VEHICLE_ID}_test2_$(DATE).moos${END}${TEST";

        let mut lexer = Lexer::new(input);
        //scan_keywords_and_values(input, &mut tokens);
        let expected_tokens = vec![
            Token::ValueString("targ_"),
            Token::PlugUpperVariable("VEHICLE_NAME"),
            Token::ValueString("_test1_"),
            Token::EnvVariable("VEHICLE_ID"),
            Token::ValueString("_test2_"),
            Token::PlugVariable("DATE"),
            Token::ValueString(".moos"),
            Token::EnvVariable("END"),
            Token::PartialEnvVariable("TEST"),
        ];

        check_tokens(&mut lexer, expected_tokens);

        let input =
            "targ_%(VEHICLE_NAME)_test1_${VEHICLE_ID}_test2_$(DATE).moos${END}${TEST//Comment}";

        let mut lexer = Lexer::new(input);
        //scan_keywords_and_values(input, &mut tokens);
        let expected_tokens = vec![
            Token::ValueString("targ_"),
            Token::PlugUpperVariable("VEHICLE_NAME"),
            Token::ValueString("_test1_"),
            Token::EnvVariable("VEHICLE_ID"),
            Token::ValueString("_test2_"),
            Token::PlugVariable("DATE"),
            Token::ValueString(".moos"),
            Token::EnvVariable("END"),
            Token::PartialEnvVariable("TEST"),
            Token::Comment("Comment}"),
        ];

        check_tokens(&mut lexer, expected_tokens);

        let input =
            "targ_//%(VEHICLE_NAME)_test1_${VEHICLE_ID}_test2_$(DATE).moos${END}${TEST//Comment}";

        let mut lexer = Lexer::new(input);
        //scan_keywords_and_values(input, &mut tokens);
        let expected_tokens = vec![
            Token::ValueString("targ_"),
            Token::Comment(
                "%(VEHICLE_NAME)_test1_${VEHICLE_ID}_test2_$(DATE).moos${END}${TEST//Comment}",
            ),
        ];

        check_tokens(&mut lexer, expected_tokens);
    }

    #[test]
    fn test_listener() {
        use crate::moos;
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        struct SemanticToken {
            token_type: i32,
            modifier: i32,
            start_loc: Location,
            end_loc: Location,
        }

        struct TokenCollector {
            tokens: Vec<SemanticToken>,
        }

        impl TokenListener for TokenCollector {
            fn handle_token(&mut self, token: &Token, start_loc: &Location, end_loc: &Location) {
                match token {
                    Token::Comment(_comment) => {
                        self.tokens.push(SemanticToken {
                            token_type: 0,
                            modifier: 1,
                            start_loc: *start_loc,
                            end_loc: *end_loc,
                        });
                    }
                    Token::BlockKeyword(keyword) => {
                        self.tokens.push(SemanticToken {
                            token_type: 3,
                            modifier: 5,
                            start_loc: *start_loc,
                            end_loc: *end_loc,
                        });
                    }
                    _ => {
                        log::debug!("Unhandled token: {:?}", token)
                    }
                }
            }
        }

        let input = r#"
        //------------------------------------------
        // uMemWatch config block

        ProcessConfig = uMemWatch
        {
          AppTick   = $(POP) // Test
          CommsTick = 4

          absolute_time_gap = 1   // In Seconds, Default is 4
          log_path = "/home/user/tmp"

          watch_only = pHelmIvP,pMarineViewer
        }
        "#;

        let input = r#"
        //------------------------------------------
        // uMemWatch config block

        TimeWarp = 10

        "#;

        let mut token_collector = TokenCollector { tokens: vec![] };

        let mut lexer = Lexer::new(input);
        lexer.add_listener(&mut token_collector);

        while let Some(Ok((_, token, _))) = lexer.next() {
            trace!("Parser Token: {:?}", token);
        }

        lexer = Lexer::new(input);
        let mut state = State::default();
        let result = moos::LinesParser::new().parse(&mut state, input, lexer);
        if let Err(e) = &result {
            trace!("Lexer Error: {:?}", e);
        }
        assert!(result.is_ok());
        trace!("Tokens: ");
        for t in &token_collector.tokens {
            trace!("  {:?}", t);
        }
    }

    #[test]
    fn test_whitespace() {
        let input = r#"  TestValue   = "This is a Quote" //   Comments are trimmed   "#;

        let mut lexer = Lexer::new(input);
        let expected_tokens = vec![
            //Token::Whitespace("  "),
            Token::ValueString("TestValue"),
            //Token::Whitespace("   "),
            Token::AssignOp,
            //Token::Whitespace(" "),
            Token::Quote("This is a Quote"),
            //Token::Whitespace(" "),
            Token::Comment("Comments are trimmed"), //
        ];
        check_tokens(&mut lexer, expected_tokens);
    }
}
