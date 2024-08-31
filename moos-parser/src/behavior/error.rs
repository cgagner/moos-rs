use crate::lexers::Location;
use crate::TreeStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehaviorParseError {
    pub kind: BehaviorParseErrorKind,
    pub loc_start: Location,
    pub loc_end: Location,
}

impl BehaviorParseError {
    pub fn new(
        kind: BehaviorParseErrorKind,
        loc_start: Location,
        loc_end: Location,
    ) -> BehaviorParseError {
        BehaviorParseError {
            kind,
            loc_start,
            loc_end,
        }
    }
    pub fn new_missing_trailing(
        c: char,
        loc_start: Location,
        loc_end: Location,
    ) -> BehaviorParseError {
        BehaviorParseError {
            kind: BehaviorParseErrorKind::MissingTrailing(c),
            loc_start,
            loc_end,
        }
    }

    pub fn new_unexpected_comment(
        comment: &str,
        loc_start: Location,
        loc_end: Location,
    ) -> BehaviorParseError {
        BehaviorParseError {
            kind: BehaviorParseErrorKind::UnexpectedComment(comment.into()),
            loc_start,
            loc_end,
        }
    }

    pub fn new_unexpected_assignment(loc_start: Location, loc_end: Location) -> BehaviorParseError {
        BehaviorParseError {
            kind: BehaviorParseErrorKind::UnexpectedAssignment,
            loc_start,
            loc_end,
        }
    }

    pub fn new_unexpected_symbol(c: char, loc_end: Location) -> BehaviorParseError {
        BehaviorParseError {
            kind: BehaviorParseErrorKind::UnexpectedSymbol(c),
            loc_start: loc_end,
            loc_end,
        }
    }
    pub fn new_missing_endif(loc_start: Location, loc_end: Location) -> BehaviorParseError {
        BehaviorParseError {
            kind: BehaviorParseErrorKind::MissingEndIf,
            loc_start,
            loc_end,
        }
    }
    pub fn new_missing_new_line(loc_start: Location, loc_end: Location) -> BehaviorParseError {
        BehaviorParseError {
            kind: BehaviorParseErrorKind::MissingNewLine,
            loc_start,
            loc_end,
        }
    }
    pub fn new_unknown_macro(loc_start: Location, macro_name: &str) -> BehaviorParseError {
        BehaviorParseError {
            kind: BehaviorParseErrorKind::UnknownMacro(macro_name.into()),
            loc_start,
            loc_end: Location {
                line: loc_start.line,
                index: loc_start.index + (macro_name.len() as u32) + 1_u32,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BehaviorParseErrorKind {
    MissingEndIf,
    MissingTrailing(char),
    MissingNewLine,
    UnexpectedAssignment,
    UnexpectedComment(TreeStr),
    UnexpectedSymbol(char),
    UnknownMacro(TreeStr),
}
