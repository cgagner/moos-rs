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
    LeftAngleBracket,
    RightAngleBracket,
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
        let mut tokens = self.iter.next();
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
        }
        self.push_token(i, Token::OrOperator, i + 2);
        self.previous_index = self.get_safe_index(i + 2);

        // Consume tokens until the next token is a non-white space or we reach
        // the end of the file
        while let Some(((current_i, _current_c), (_next_i, next_c))) = tokens {
            match next_c {
                ' ' | '\t' => tokens = self.iter.next(),
                _ => {
                    self.previous_index = self.get_safe_index(current_i);
                    break;
                }
            }
        }
    }

    fn tokenize_and_operator(&mut self, i: usize) {
        let mut tokens = self.iter.next();
        if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
            if !unhandled.is_empty() {
                self.scan_value(unhandled, prev_i);
            }
        }

        self.push_token(i, Token::AndOperator, i + 2);
        self.previous_index = self.get_safe_index(i + 2);

        // Consume tokens until the next token is a non-white space or we reach
        // the end of the file
        while let Some(((current_i, _current_c), (_next_i, next_c))) = tokens {
            match next_c {
                ' ' | '\t' => tokens = self.iter.next(),
                _ => {
                    self.previous_index = self.get_safe_index(current_i);
                    break;
                }
            }
        }
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
        let (token, _next_index) = if let Some(((ii, cc), (_iii, _ccc))) =
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

        let (is_ifndef, is_include) = match token {
            Token::MacroInclude => (false, true),
            Token::MacroIfNotDef => (true, false),
            _ => (false, false),
        };

        let has_conditions = match token {
            Token::MacroIfDef | Token::MacroElseIfDef => true,
            // #ifndef doesn't really support conditions, but we will handle
            // that in the parser. For now, enable the tokenization of the
            // && and || operators so we can throw an in the parser.
            Token::MacroIfNotDef => true,
            _ => false,
        };

        let has_comments = match token {
            Token::MacroElse | Token::MacroEndIf => true,
            _ => false,
        };

        let mut has_whitespace = match token {
            Token::MacroDefine | Token::MacroIfDef | Token::MacroElseIfDef => true,
            Token::MacroIfNotDef => true,
            _ => false,
        };

        let mut found_token_before_space = false;

        while let Some(((i, c), (_ii, cc))) = self.iter.find(|&((_i, c), (_ii, cc))| {
            c == '\n'
                || c == '"'
                || (has_whitespace && (c == ' ' || c == '\t')) // Whitespace
                || (has_comments && (c == '/' && cc == '/')) // Comment
                || (c == '$' && cc == '(') // Plug variable
                || (c == '%' && cc == '(') // Plug Upper Variable
                || (has_conditions && c == '|' && cc == '|') // Or operator
                || (has_conditions && c == '&' && cc == '&' ) // And operator
                || (is_include && (c == ' ' || c == '\t') && cc == '<') // Handle include tags
        }) {
            match c {
                c if is_include && (c == ' ' || c == '\t') && cc == '<' => {
                    // Handle include - This needs to happen before handling
                    // the spaces below.
                    let found_include_tag = self.tokenize_include_tag(i);
                    if found_include_tag {
                        return;
                    }
                }
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
                    if has_whitespace {
                        found_token_before_space = true;
                    }

                    if self.found_assign_op {
                        self.trim_start = false;
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
                    if has_whitespace {
                        found_token_before_space = true;
                    }
                    if self.found_assign_op {
                        self.trim_start = false;
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
                    if has_whitespace {
                        found_token_before_space = true;
                    }
                    if self.found_assign_op {
                        self.trim_start = false;
                    }
                }
                '|' => {
                    self.trim_end = true;
                    self.tokenize_or_operator(i);
                    has_whitespace = true;
                    self.trim_start = true;
                    self.trim_end = false;
                    self.found_assign_op = false;
                }
                '&' => {
                    self.trim_end = true;
                    self.tokenize_and_operator(i);
                    has_whitespace = true;
                    self.trim_start = true;
                    self.trim_end = false;
                    self.found_assign_op = false;
                }
                c if has_whitespace && (c == ' ' || c == '\t') => {
                    if !is_ifndef {
                        self.found_assign_op = true; // Enables parsing primitives
                    }
                    self.trim_end = true;
                    if let Some((prev_i, unhandled)) = self.get_unhandled_string(i, true) {
                        if !unhandled.is_empty() {
                            self.scan_value(unhandled, prev_i);
                            found_token_before_space = true;
                        }
                    }
                    if found_token_before_space && !is_ifndef {
                        self.push_token(i, Token::Space, i + 1);
                        has_whitespace = false;
                        found_token_before_space = false;
                    }
                    self.trim_start = true;
                    self.trim_end = false;
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

    fn tokenize_include_tag(&mut self, index: usize) -> bool {
        // The include tag is a space followed by a '<', then a tag, then
        // a '>' and the end of the line. If it does not follow that format,
        // the characters should be treated as part of the include path.

        // NOTE: We do NOT handle nested tags (E.G. <<my_tag>>) because
        // nsplug doesn't care. It finds the first '<' then the last '>'.

        // Make a clone of the iterator so we can look forward to the end of
        // the line to see if this is a valid tag
        let mut local_iter = self.iter.clone();

        let mut right_bracket_location: Option<usize> = None;
        let mut new_line_index: Option<usize> = None;

        // Search until we find the end of the line. Mark the location of the
        // last '>'.
        while let Some(((i, c), (_ii, _cc))) = local_iter.find(|&((_i, c), (_ii, _cc))| {
            c == '\n' // New line
            || c == '>' // Right angle bracket
        }) {
            match c {
                '>' => right_bracket_location = Some(i),
                '\n' => {
                    new_line_index = Some(i);
                    break;
                }
                _ => {}
            }
        }

        if let Some(right_bracket_location) = right_bracket_location {
            let remaining = if let Some(i) = new_line_index {
                // Up to, but not including the new line
                &self.input[index + 1..i]
            } else {
                // Until the end of the file
                &self.input[index + 1..]
            }
            .trim();

            // Check that the right bracket is the last character in the trimmed
            // string.
            if remaining.len() < 2 && !remaining.ends_with(">") {
                return false;
            }

            // Check that there isn't any whitespace in the remaining
            if let Some(_i) = remaining.find(char::is_whitespace) {
                return false;
            }

            // We have found a tag.
            // Push unhandled before the tag
            self.trim_end = true;
            if let Some((prev_i, unhandled)) = self.get_unhandled_string(index, true) {
                if !unhandled.is_empty() {
                    self.scan_value(unhandled, prev_i);
                }
            }

            // Push the left angle bracket
            self.push_token(index + 1, Token::LeftAngleBracket, index + 2);

            // Push the tag as a value string
            self.push_token(
                index + 2,
                Token::ValueString(&remaining[1..remaining.len() - 1]),
                right_bracket_location,
            );

            // Push the right angle bracket
            self.push_token(
                right_bracket_location,
                Token::RightAngleBracket,
                right_bracket_location + 1,
            );

            // If we found a new line, EOL
            if let Some(i) = new_line_index {
                self._handle_new_line(i);
            } else {
                self.previous_index = None;
            }

            // Update our iterator to our local copy
            self.iter = local_iter;

            return true;
        } else {
            return false;
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
