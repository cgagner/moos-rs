use crate::lexers::Location;
use crate::TreeStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlugParseError {
    pub kind: PlugParseErrorKind,
    pub loc_start: Location,
    pub loc_end: Location,
}

impl PlugParseError {
    pub fn new(kind: PlugParseErrorKind, loc_start: Location, loc_end: Location) -> PlugParseError {
        PlugParseError {
            kind,
            loc_start,
            loc_end,
        }
    }
    pub fn new_missing_trailing(c: char, loc_start: Location, loc_end: Location) -> PlugParseError {
        PlugParseError {
            kind: PlugParseErrorKind::MissingTrailing(c),
            loc_start,
            loc_end,
        }
    }

    pub fn new_unexpected_comment(
        comment: &str,
        loc_start: Location,
        loc_end: Location,
    ) -> PlugParseError {
        PlugParseError {
            kind: PlugParseErrorKind::UnexpectedComment(comment.into()),
            loc_start,
            loc_end,
        }
    }

    pub fn new_unexpected_symbol(c: char, loc_end: Location) -> PlugParseError {
        PlugParseError {
            kind: PlugParseErrorKind::UnexpectedSymbol(c),
            loc_start: loc_end,
            loc_end,
        }
    }
    pub fn new_missing_endif(loc_start: Location, loc_end: Location) -> PlugParseError {
        PlugParseError {
            kind: PlugParseErrorKind::MissingEndIf,
            loc_start,
            loc_end,
        }
    }
    pub fn new_missing_new_line(loc_start: Location, loc_end: Location) -> PlugParseError {
        PlugParseError {
            kind: PlugParseErrorKind::MissingNewLine,
            loc_start,
            loc_end,
        }
    }
    pub fn new_unknown_macro(loc_start: Location, macro_name: &str) -> PlugParseError {
        PlugParseError {
            kind: PlugParseErrorKind::UnknownMacro(macro_name.into()),
            loc_start,
            loc_end: Location {
                line: loc_start.line,
                index: loc_start.index + (macro_name.len() as u32) + 1_u32,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlugParseErrorKind {
    MissingEndIf,
    MissingTrailing(char),
    MissingNewLine,
    UnexpectedComment(TreeStr),
    UnexpectedSymbol(char),
    UnknownMacro(TreeStr),
}
