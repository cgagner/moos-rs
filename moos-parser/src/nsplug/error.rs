use crate::lexers::Location;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PlugParseError<'input> {
    pub kind: PlugParseErrorKind<'input>,
    pub loc_start: Location,
    pub loc_end: Location,
}

impl<'input> PlugParseError<'input> {
    pub fn new(kind: PlugParseErrorKind, loc_start: Location, loc_end: Location) -> PlugParseError {
        PlugParseError {
            kind,
            loc_start,
            loc_end,
        }
    }
    pub fn new_missing_trailing(
        c: char,
        loc_start: Location,
        loc_end: Location,
    ) -> PlugParseError<'input> {
        PlugParseError {
            kind: PlugParseErrorKind::MissingTrailing(c),
            loc_start,
            loc_end,
        }
    }

    pub fn new_unexpected_comment(
        comment: &'input str,
        loc_start: Location,
        loc_end: Location,
    ) -> PlugParseError<'input> {
        PlugParseError {
            kind: PlugParseErrorKind::UnexpectedComment(comment),
            loc_start,
            loc_end,
        }
    }

    pub fn new_unexpected_symbol(c: char, loc_end: Location) -> PlugParseError<'input> {
        PlugParseError {
            kind: PlugParseErrorKind::UnexpectedSymbol(c),
            loc_start: loc_end,
            loc_end,
        }
    }
    pub fn new_missing_new_line(loc_start: Location, loc_end: Location) -> PlugParseError<'input> {
        PlugParseError {
            kind: PlugParseErrorKind::MissingNewLine,
            loc_start: loc_start,
            loc_end,
        }
    }
    pub fn new_unknown_macro(
        loc_start: Location,
        macro_name: &'input str,
    ) -> PlugParseError<'input> {
        PlugParseError {
            kind: PlugParseErrorKind::UnknownMacro(macro_name),
            loc_start,
            loc_end: Location {
                line: loc_start.line,
                index: loc_start.index + (macro_name.len() as u32) + 1_u32,
            },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlugParseErrorKind<'input> {
    MissingTrailing(char),
    MissingNewLine,
    UnexpectedComment(&'input str),
    UnexpectedSymbol(char),
    UnknownMacro(&'input str),
}
