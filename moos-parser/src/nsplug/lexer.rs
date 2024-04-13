use super::error::PlugParseError;
use crate::lexers::{scan_bool, scan_float, scan_integer, Location};

use core::cmp::max;
use core::str;
use core::str::CharIndices;
use lalrpop_util::ErrorRecovery;
use std::collections::{HashMap, VecDeque};
use std::iter::{Chain, Repeat, Skip};

pub type Spanned<Token, Loc, Error> = Result<(Loc, Token, Loc), Error>;
pub type TokenQueue<'input> = VecDeque<Spanned<Token<'input>, Location, PlugParseError<'input>>>;

#[derive(Debug, Default, Clone)]
pub struct State<'input> {
    pub errors: Vec<ErrorRecovery<Location, Token<'input>, PlugParseError<'input>>>,
    pub defines: HashMap<String, String>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Token<'input> {
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
    LeftAngleBracket,
    RightAngleBracket,
    WhiteSpace(&'input str),
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
     */
    #[inline]
    fn get_unhandled_string(&self, index: usize) -> Option<(usize, &'input str)> {
        if let Some(prev_i) = self.previous_index {
            let start_index = prev_i;

            let unhandled = &self.input[start_index..index];

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
        self.previous_index = self.get_safe_index(i + 1);
    }

    fn scan_value(&mut self, line: &'input str, line_index: usize) {
        if line.is_empty() {
            return;
        }

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

        self.push_token(
            line_index,
            Token::ValueString(&line),
            line_index + line.len(),
        );
    }

    fn tokenize_or_operator(&mut self, i: usize) {
        let _tokens = self.iter.next();
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
        }
        self.push_token(i, Token::OrOperator, i + 2);
        self.previous_index = self.get_safe_index(i + 2);
    }

    fn tokenize_and_operator(&mut self, i: usize) {
        let _tokens = self.iter.next();
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
        }

        self.push_token(i, Token::AndOperator, i + 2);
        self.previous_index = self.get_safe_index(i + 2);
    }

    fn tokenize_include_bracket(&mut self, i: usize, c: char) {
        let token = match c {
            '<' => Token::LeftAngleBracket,
            '>' => Token::RightAngleBracket,
            _ => return,
        };

        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
        }

        self.push_token(i, token, i + 1);
        self.previous_index = self.get_safe_index(i + 1);
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

        // Make sure the current line starts with nothing but whitespace before
        // the '#'
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i) {
            if !unhandled.trim().is_empty() {
                return;
            }
            // Push the indent as a whitespace token.
            self.push_token(prev_i, Token::WhiteSpace(unhandled), i);
            self.previous_index = self.get_safe_index(i);
        }
        // [endif|else] <-- Must have a space after endif
        // include [quote|string|variable]
        // define [key|variable] [value|variable]
        // [ifdef|elseifdef] [condition] [|| &&] [condition]
        // ifndef [key|variable]
        // [condition] => [string|quote|variable]

        // Find the macro by looking for the next whitespace or newline
        let (token, _next_index) = if let Some(((_ii, _cc), (iii, ccc))) =
            self.iter.find(|&((_ii, _cc), (_iii, ccc))| {
                ccc == ' ' || ccc == '\t'  // Whitespace
                || ccc == '\n' // Newline
            }) {
            // Get the line
            let line = &self.input[i + 1..iii];
            let token = Self::get_macro_token(line);
            self.push_token(i, token, iii);
            self.previous_index = self.get_safe_index(iii);
            match ccc {
                '\n' => {
                    // Handle this back in the main tokenize method
                    return;
                }
                _ => {}
            }
            (token, iii)
        } else {
            // If we get here, we reached the end of the file.
            let line = &self.input[i + 1..];
            let token = Self::get_macro_token(line);
            self.push_token(i, token, self.input.len());
            self.previous_index = None;
            return;
        };

        let is_include = match token {
            Token::MacroInclude => true,
            _ => false,
        };

        let has_conditions = match token {
            Token::MacroIfDef | Token::MacroElseIfDef => true,
            // #ifndef doesn't really support conditions, but we will handle
            // that in the parser. For now, enable the tokenization of the
            // && and || operators so we can throw an in the parser.
            Token::MacroIfNotDef => true,
            _ => false,
        };

        let has_whitespace = match token {
            Token::MacroDefine | Token::MacroIfDef | Token::MacroElseIfDef => true,
            Token::MacroIfNotDef => true,
            _ => true,
        };

        while let Some(((i, c), (_ii, cc))) = self.iter.find(|&((_i, c), (_ii, cc))| {
            c == '\n'
                || c == '"'
                || (has_whitespace && (c == ' ' || c == '\t')) // Whitespace
                || (c == '$' && cc == '(') // Plug variable
                || (c == '%' && cc == '(') // Plug Upper Variable
                || (has_conditions && c == '|' && cc == '|') // Or operator
                || (has_conditions && c == '&' && cc == '&' ) // And operator
                || (is_include && c == '<') // Handle include tags
                || (is_include && c == '>') // Handle include tags
        }) {
            match c {
                c if is_include && (c == '<' || c == '>') => {
                    self.tokenize_include_bracket(i, c);
                }
                '\n' => {
                    self.tokenize_new_line(i, false);
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
                    self.tokenize_or_operator(i);
                }
                '&' => {
                    self.tokenize_and_operator(i);
                }
                c if has_whitespace && (c == ' ' || c == '\t') => {
                    self.tokenize_whitespace(i, cc);
                }
                _ => {}
            }
        }

        // Should only get in here if we have reached the end of the input.
        // If so, check that there isn't some straggling unhandled string.
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(self.input.len()) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
            self.previous_index = self.get_safe_index(self.input.len());
        }
    }

    fn tokenize_new_line(&mut self, i: usize, drop_unhandled: bool) {
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i) {
            if !unhandled.is_empty() && !drop_unhandled {
                self.scan_value(unhandled, prev_i);
            }
        }
        self._handle_new_line(i);
        // Break out of the tokenize for-loop after each line
    }

    fn tokenize_whitespace(&mut self, i: usize, next_c: char) {
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
        }

        // If the next Character is a non-whitespace character, we can just
        // push the whitespace token onto the queue and move on.
        if next_c != ' ' && next_c != '\t' {
            let text = &self.input[i..i + 1];
            self.push_token(i, Token::WhiteSpace(text), i + 1);
            self.previous_index = self.get_safe_index(i + 1);
            return;
        }

        // Go until the next character is non-whitespace  or the end of the file
        while let Some(((ii, _cc), (_iii, _ccc))) = self
            .iter
            .find(|&((_ii, _cc), (_iii, ccc))| ccc != ' ' && ccc != '\t')
        {
            let text = &self.input[i..ii + 1];
            self.push_token(i, Token::WhiteSpace(text), ii + 1);
            self.previous_index = self.get_safe_index(ii + 1);
            return;
        }

        // Reached the end of the input
        let text = &self.input[i..];
        self.push_token(i, Token::WhiteSpace(text), self.input.len());
        self.previous_index = None;
    }

    /// Tokenize a quote.
    /// Returns true if a full quote is found; false if the end of the line
    /// or end of the file is reached without finding the matching quote.
    fn tokenize_quote(&mut self, i: usize) -> bool {
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
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
                    if let Some((prev_i, unhandled)) = self.get_unhandled_string(ii) {
                        if !unhandled.is_empty() {
                            self.scan_value(unhandled, prev_i);
                        }
                    }
                    self.push_token(ii, Token::QuoteEnd, ii + 1);
                    self.previous_index = self.get_safe_index(ii + 1);
                    return true;
                }
                // Handle Variables inside of quotes
                cc if (cc == '$' && ccc == '(') => {
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
                    if let Some((prev_i, unhandled)) = self.get_unhandled_string(ii) {
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
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(self.input.len()) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
        }
        self.previous_index = None;
        return false;
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
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
                self.start_of_line = false;
            }
        }
        // Find the matching end brace or paren or end of line

        while let Some(((ii, cc), (_iii, _ccc))) = self
            .iter
            .find(|&((_ii, cc), (_iii, _ccc))| cc == '\n' || cc == ')')
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
        //   2. Plug variable
        //   3. Plug upper variable
        //   4. Macro
        //
        // Ignore other tokens

        while let Some(((i, c), (_ii, cc))) = self.iter.find(|&((_i, c), (_ii, cc))| {
            c == '\n'
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
