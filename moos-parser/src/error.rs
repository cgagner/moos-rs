use crate::lexers::Location;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MoosParseError<'input> {
    pub kind: MoosParseErrorKind<'input>,
    pub loc_start: Location,
    pub loc_end: Location,
}

impl<'input> MoosParseError<'input> {
    pub fn new(kind: MoosParseErrorKind, loc_start: Location, loc_end: Location) -> MoosParseError {
        MoosParseError {
            kind,
            loc_start,
            loc_end,
        }
    }
    pub fn new_missing_trailing(c: char, loc_end: Location) -> MoosParseError<'input> {
        MoosParseError {
            kind: MoosParseErrorKind::MissingTrailing(c),
            loc_start: loc_end,
            loc_end,
        }
    }
    pub fn new_unexpected_symbol(c: char, loc_end: Location) -> MoosParseError<'input> {
        MoosParseError {
            kind: MoosParseErrorKind::UnexpectedSymbol(c),
            loc_start: loc_end,
            loc_end,
        }
    }
    pub fn new_missing_new_line(loc_start: Location, loc_end: Location) -> MoosParseError<'input> {
        MoosParseError {
            kind: MoosParseErrorKind::MissingNewLine,
            loc_start: loc_start,
            loc_end,
        }
    }
    pub fn new_unknown_macro(
        loc_start: Location,
        macro_name: &'input str,
    ) -> MoosParseError<'input> {
        MoosParseError {
            kind: MoosParseErrorKind::UnknownMacro(macro_name),
            loc_start,
            loc_end: Location {
                line: loc_start.line,
                index: loc_start.index + macro_name.len() as u32,
            },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MoosParseErrorKind<'input> {
    MissingTrailing(char),
    MissingNewLine,
    InvalidConfigBlock,
    UnexpectedSymbol(char),
    UnknownMacro(&'input str),
}
