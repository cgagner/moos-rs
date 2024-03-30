// TODO: Need to handle partial variable inside of partial quotes.
// This is a bit trickier because we need to push the tokens into the queue
// in the correct order so they appear in the parser correctly. However,
// The token listener needs these tokens in the opposite order.
// E.G.
//
// TODO: Variables cannot contain spaces. This is because of the way the
// #define macro is handled.
//
// The token listener approach is flawed. It does not take semantics into
// account.

use super::error::PlugParseError;
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
pub type TokenQueue<'input> = VecDeque<Spanned<Token<'input>, Location, PlugParseError<'input>>>;

#[derive(Debug, Default, Clone)]
pub struct State<'input> {
    pub errors: Vec<ErrorRecovery<Location, Token<'input>, PlugParseError<'input>>>,
    pub defines: HashMap<String, String>,
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
    Space,
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

    /**
     * Drop the unhandled string from an the previous index to the current index
     * of a string.
     *
     * # Parameters:
     * * `index`: Current index
     */
    #[inline]
    fn drop_unhandled_string(&mut self, index: usize) -> Option<(usize, &'input str)> {
        let result = if let Some(prev_i) = self.previous_index {
            let start_index = prev_i;
            let unhandled = &self.input[start_index..index];
            if unhandled.is_empty() {
                None
            } else {
                Some((start_index, unhandled))
            }
        } else {
            None
        };

        if index > 0 {
            self.previous_index = self.get_safe_index(index);
        } else {
            self.previous_index = None;
        }

        return result;
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

    fn tokenize_or_operator(&mut self, i: usize) {
        self.iter.next();
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
            self.previous_index = self.get_safe_index(i + 2);
        }

        self.push_token(i, Token::OrOperator, i + 2);
    }

    fn tokenize_and_operator(&mut self, i: usize) {
        self.iter.next();
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
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
        // [endif|else] - [comment] <-- Must have a space after endif
        // include [quote|string|variable] [comment]
        // define [key|variable] [value|variable]
        // [ifdef|elseifdef] [condition] [|| &&] [condition]
        // ifndef [key|variable]
        // [condition] => [string|quote|variable]

        // TODO: We probably should remove parsing comments

        // Find the macro by looking for the next whitespace, newline, or comment
        let (token, next_index) = if let Some(((ii, cc), (_iii, _ccc))) =
            self.iter.find(|&((_ii, cc), (_iii, ccc))| {
                cc == ' ' || cc == '\t'  // Whitespace
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
                || (has_whitespace && (c == ' ' || c == '\t')) // Whitespace
                || (c == '/' && cc == '/') // Comment
                || (c == '$' && cc == '(') // Plug variable
                || (c == '%' && cc == '(') // Plug Upper Variable
                || (has_conditions && c == '|' && cc == '|') // Or operator
                || (has_conditions && c == '&' && cc == '&' ) // And operator
        }) {
            match c {
                '\n' => {
                    self.tokenize_new_line(i, false);
                    return;
                }
                '/' => {
                    self.tokenize_comment(i);
                    return;
                }
                '"' => {
                    let found_quote = self.tokenize_quote(i);
                    if !found_quote {
                        return;
                    }
                }
                c if (c == '$' && cc == '(') => {
                    let found_variable =
                        self.tokenize_variable(i, c, cc, |text: &'input str, is_partial: bool| {
                            if !is_partial {
                                Token::PlugVariable(text)
                            } else {
                                Token::PartialPlugVariable(text)
                            }
                        });
                    if !found_variable {
                        return;
                    }
                }
                c if (c == '%' && cc == '(') => {
                    let found_variable =
                        self.tokenize_variable(i, c, cc, |text: &'input str, is_partial: bool| {
                            if !is_partial {
                                Token::PlugUpperVariable(text)
                            } else {
                                Token::PartialPlugUpperVariable(text)
                            }
                        });
                    if !found_variable {
                        return;
                    }
                }
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
                ' ' | '\t' => {
                    self.found_assign_op = true; // Enables parsing primitives
                    self.trim_end = true;
                    if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
                        if !unhandled.is_empty() {
                            self.scan_value(unhandled, prev_i);
                            has_whitespace = false;
                            self.push_token(i, Token::Space, i + 1);
                        }
                        self.previous_index = self.get_safe_index(i + 1);
                    }
                    self.trim_start = true;

                    self.previous_index = self.get_safe_index(i + 1);
                }
                _ => {}
            }
        }

        // Should only get in here if we have reached the end of the input.
        // If so, check that there isn't some straggling unhandled string.
        self.trim_end = true;
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(self.input.len(), true) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
            self.previous_index = self.get_safe_index(self.input.len());
        }
    }

    fn tokenize_new_line(&mut self, i: usize, drop_unhandled: bool) {
        // Trim up to the new line
        self.trim_end = true;
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() && !drop_unhandled {
                self.scan_value(unhandled, prev_i);
            }
        }
        self._handle_new_line(i);
        // Break out of the tokenize for-loop after each line
    }

    /// Tokenize a quote.
    /// Returns true if a full quote is found; false if the end of the line
    /// or end of the file is reached without finding the matching quote.
    fn tokenize_quote(&mut self, i: usize) -> bool {
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
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
                            self.scan_value(unhandled, prev_i);
                        }
                    }
                    self.push_token(ii, Token::QuoteEnd, ii + 1);
                    self.trim_start = false;
                    self.previous_index = self.get_safe_index(ii + 1);
                    return true;
                }
                // Handle Variables inside of quotes
                cc if (cc == '$' && ccc == '(') => {
                    // TODO: Add Quote Begin
                    let found_variable = self.tokenize_variable(
                        ii,
                        cc,
                        ccc,
                        |text: &'input str, is_partial: bool| {
                            if !is_partial {
                                Token::PlugVariable(text)
                            } else {
                                Token::PartialPlugVariable(text)
                            }
                        },
                    );
                    if !found_variable {
                        return false;
                    }
                }
                cc if (cc == '%' && ccc == '(') => {
                    // TODO: Add Quote Begin
                    let found_variable = self.tokenize_variable(
                        ii,
                        cc,
                        ccc,
                        |text: &'input str, is_partial: bool| {
                            if !is_partial {
                                Token::PlugUpperVariable(text)
                            } else {
                                Token::PartialPlugUpperVariable(text)
                            }
                        },
                    );
                    // If the variable was not found, return.
                    if !found_variable {
                        return false;
                    }
                }
                '\n' => {
                    // Push any unhandled tokens before the End of line
                    if let Some((prev_i, unhandled)) = self.get_unhandled_string(ii, false) {
                        if !unhandled.is_empty() {
                            self.scan_value(unhandled, prev_i);
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
                self.scan_value(unhandled, prev_i);
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
                self.scan_value(unhandled, prev_i);
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

    /// Tokenize a Plug variable
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
                self.scan_value(unhandled, prev_i);
                self.start_of_line = false;
                self.trim_start = false;
            }
        }
        // Find the matching end brace or paren or end of line

        while let Some(((ii, cc), (_iii, _ccc))) = self
            .iter
            .find(|&((_ii, cc), (_iii, ccc))| cc == '\n' || cc == ')')
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
        //   2. Comment // Deprecated. NSPlug does not really support comments
        //   3. Plug variable
        //   4. Plug upper variable
        //   5. Macro
        //
        // Ignore other tokens

        while let Some(((i, c), (_ii, cc))) = self.iter.find(|&((_i, c), (_ii, cc))| {
            c == '\n'
                // NSPlug does not really support comments
                // || (c == '/' && cc == '/') // Comment
                || (c == '$' && cc == '(') // Plug variable
                || (c == '%' && cc == '(') // Plug Upper Variable
                || (c == '#') // Macro
        }) {
            match c {
                '\n' => {
                    self.tokenize_new_line(i, true);
                    // Break out of the tokenize for-loop after each line
                    break;
                }
                // NSPlug does not really support comments.
                //'/' => self.tokenize_comment(i),
                c if (c == '$' && cc == '(') => {
                    // drop the unhandled tokens before this because we are not
                    // on a macro line
                    self.drop_unhandled_string(i);
                    self.tokenize_variable(i, c, cc, |text: &'input str, is_partial: bool| {
                        if !is_partial {
                            Token::PlugVariable(text)
                        } else {
                            Token::PartialPlugVariable(text)
                        }
                    });
                }
                c if (c == '%' && cc == '(') => {
                    // drop the unhandled tokens before this because we are not
                    // on a macro line
                    self.drop_unhandled_string(i);
                    self.tokenize_variable(i, c, cc, |text: &'input str, is_partial: bool| {
                        if !is_partial {
                            Token::PlugUpperVariable(text)
                        } else {
                            Token::PartialPlugUpperVariable(text)
                        }
                    });
                }
                '#' => self.tokenize_macro(i),
                _ => {}
            }
        }

        // NOTE: There could still be tokens to be parse, but we don't care
        // about them.
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token<'input>, Location, PlugParseError<'input>>;
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
