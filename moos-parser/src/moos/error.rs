use crate::lexers::Location;
use crate::TreeStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoosParseError {
    pub kind: MoosParseErrorKind,
    pub loc_start: Location,
    pub loc_end: Location,
}

impl MoosParseError {
    pub fn new(kind: MoosParseErrorKind, loc_start: Location, loc_end: Location) -> MoosParseError {
        MoosParseError {
            kind,
            loc_start,
            loc_end,
        }
    }
    pub fn new_missing_trailing(c: char, loc_start: Location, loc_end: Location) -> MoosParseError {
        MoosParseError {
            kind: MoosParseErrorKind::MissingTrailing(c),
            loc_start,
            loc_end,
        }
    }

    pub fn new_unexpected_comment(
        comment: &str,
        loc_start: Location,
        loc_end: Location,
    ) -> MoosParseError {
        MoosParseError {
            kind: MoosParseErrorKind::UnexpectedComment(comment.into()),
            loc_start,
            loc_end,
        }
    }

    pub fn new_unexpected_symbol(c: char, loc_end: Location) -> MoosParseError {
        MoosParseError {
            kind: MoosParseErrorKind::UnexpectedSymbol(c),
            loc_start: loc_end,
            loc_end,
        }
    }
    pub fn new_missing_endif(loc_start: Location, loc_end: Location) -> MoosParseError {
        MoosParseError {
            kind: MoosParseErrorKind::MissingEndIf,
            loc_start,
            loc_end,
        }
    }
    pub fn new_missing_new_line(loc_start: Location, loc_end: Location) -> MoosParseError {
        MoosParseError {
            kind: MoosParseErrorKind::MissingNewLine,
            loc_start,
            loc_end,
        }
    }
    pub fn new_unknown_macro(loc_start: Location, macro_name: &str) -> MoosParseError {
        MoosParseError {
            kind: MoosParseErrorKind::UnknownMacro(macro_name.into()),
            loc_start,
            loc_end: Location {
                line: loc_start.line,
                index: loc_start.index + (macro_name.len() as u32) + 1_u32,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoosParseErrorKind {
    MissingEndIf,
    MissingTrailing(char),
    MissingNewLine,
    UnexpectedComment(TreeStr),
    UnexpectedSymbol(char),
    UnknownMacro(TreeStr),
}
