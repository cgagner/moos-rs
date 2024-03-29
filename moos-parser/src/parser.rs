use crate::lexers::TokenRange;
use lalrpop_util::lalrpop_mod;
use lalrpop_util::ErrorRecovery;

lalrpop_mod!(
    #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
    pub moos
); // synthesized by LALRPOP

/// TODO: Dear Future Chris: Please fix this enumeration. This should be able
/// to handle any of the tokens that can compose a value. Additionally, there
/// needs to be a collection of this enum that implements the eval method
/// to get a string.
#[derive(Debug)]
pub enum Value<'input> {
    Boolean(bool, &'input str, TokenRange),
    Integer(i64, &'input str, TokenRange),
    Float(f64, &'input str, TokenRange),
    String(&'input str, TokenRange),
    EnvVariable(&'input str, TokenRange),
    PlugVariable(&'input str, TokenRange),
    PlugUpperVariable(&'input str, TokenRange),
    PartialEnvVariable(&'input str, TokenRange),
    PartialPlugVariable(&'input str, TokenRange),
    PartialPlugUpperVariable(&'input str, TokenRange),
}

impl<'input> ToString for Value<'input> {
    fn to_string(&self) -> String {
        match *self {
            Self::Boolean(_, value_str, _)
            | Self::Integer(_, value_str, _)
            | Self::Float(_, value_str, _)
            | Self::String(value_str, _) => value_str.to_owned(),
            Self::EnvVariable(value_str, _) => {
                std::env::var(value_str).unwrap_or(format!("${{{}}}", value_str))
            }
            Self::PartialEnvVariable(value_str, _) => format!("${{{}", value_str),
            // We won't evaluate plug variables as part of this parser.
            Self::PlugVariable(value_str, _) => format!("$({})", value_str),
            Self::PlugUpperVariable(value_str, _) => format!("%({})", value_str),
            Self::PartialPlugVariable(value_str, _) => format!("$({}", value_str),
            Self::PartialPlugUpperVariable(value_str, _) => format!("%({}", value_str),
        }
    }
}

#[derive(Debug, Default)]
struct Values<'input>(Vec<Value<'input>>);

impl<'input> IntoIterator for Values<'input> {
    type Item = Value<'input>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'input> IntoIterator for &'input Values<'input> {
    type Item = &'input Value<'input>;
    type IntoIter = core::slice::Iter<'input, Value<'input>>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<'input> Values<'input> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'input, Value> {
        self.0.iter()
    }

    /// Combine all of the values into one string. If there are environment
    /// variables, those will be evaluated and replaced with their values.
    #[inline]
    pub fn eval(&self) -> String {
        let rtn = "".to_owned();
        self.0
            .iter()
            .fold(rtn, |acc, v| acc + v.to_string().as_str())
    }

    #[inline]
    pub fn first(&self) -> Option<&Value<'input>> {
        self.0.first()
    }

    #[inline]
    pub fn push(&mut self, value: Value<'input>) {
        self.0.push(value);
    }

    #[inline]
    pub fn pop(&mut self) -> Option<Value<'input>> {
        self.0.pop()
    }

    #[inline]
    pub fn last(&self) -> Option<&Value<'input>> {
        self.0.last()
    }
}

#[derive(Debug)]
pub enum MacroType<'input> {
    Define(MacroDefinition<'input>),
    Include(&'input str),
    IfDef,
    IfNotDef,
    ElseIfDef,
    Else,
    EndIf,
}

#[derive(Debug)]
pub struct MacroDefinition<'input> {
    name: &'input str,
    value: Option<Value<'input>>,
}

impl<'input> MacroDefinition<'input> {
    /// Create a new MacroDefinition
    pub fn new(name: &'input str, value: Option<Value<'input>>) -> Self {
        MacroDefinition { name, value }
    }

    pub fn eval() -> bool {
        // TODO: Implement
        false
    }
}

#[derive(Debug)]
pub enum MacroCondition<'input> {
    // Simple Definition
    Simple(MacroDefinition<'input>),
    // Disjunction Expression (a.k.a. Logical-Or)
    Disjunction(Vec<MacroDefinition<'input>>),
    // Conjunction Expression (a.k.a. Logical-And)
    Conjunction(Vec<MacroDefinition<'input>>),
    // Mixture of Disjunction and Conjunction - This is an error or false
    Mixed(Vec<MacroDefinition<'input>>),
}

#[derive(Debug)]
pub enum Line<'input> {
    Comment(&'input str),
    Define(&'input str, Value<'input>, Option<&'input str>),
    BlockBegin(&'input str, Option<&'input str>),
    BlockEnd(Option<&'input str>),
    Assignment(Vec<Value<'input>>, Vec<Value<'input>>, Option<&'input str>),
    Macro(MacroType<'input>, Option<&'input str>),
    PlugVariable(&'input str),
    PlugUpperVariable(&'input str),
    Error,
    EndOfLine,
}

// ----------------------------------------------------------------------------
// Tests
#[cfg(test)]
mod tests {

    use crate::{
        lexer::{Lexer, State},
        lexers::TokenRange,
    };

    use super::{Value, Values};

    lalrpop_mod!(
        #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
        pub moos
    ); // syntesized by LALRPOP

    #[test]
    fn test_block_newline_fail() {
        let input = r#"
        // Antler configuration  block
        ProcessConfig = ANTLER {
            MSBetweenLaunches = 200
        }
        "#;

        let mut lexer = Lexer::new(input);

        while let Some(Ok((_, token, _))) = lexer.next() {
            println!("Parser Token: {:?}", token);
        }

        lexer = Lexer::new(input);
        let mut state = State::default();
        let result = moos::LinesParser::new().parse(&mut state, input, lexer);
        println!("Result: {:?}", result);

        // // This test should fail
        // assert!(result.is_err());
        // if let Err(e) = result {
        //     assert_eq!(
        //         lalrpop_util::ParseError::User {
        //             error: crate::error::MoosParseError::new_missing_new_line(
        //                 crate::lexers::Location::new(2, 31),
        //                 crate::lexers::Location::new(2, 32),
        //             ),
        //         },
        //         e,
        //     )
        // }
    }

    #[test]
    fn test_block_newline_pass() {
        let input = r#"
        // Antler configuration  block
        ProcessConfig = ANTLER 
        {
            MSBetweenLaunches = 200
        }
        "#;

        let mut lexer = Lexer::new(input);

        while let Some(Ok((_, token, _))) = lexer.next() {
            println!("Parser Token: {:?}", token);
        }

        lexer = Lexer::new(input);
        let mut state = State::default();
        let result = moos::LinesParser::new().parse(&mut state, input, lexer);
        println!("Result: {:?}", result);

        // This test should fail
        assert!(result.is_ok());
        assert!(state.errors.is_empty());
    }

    #[test]
    fn test_line_parser() {
        let input = r#"
        define: TEST_VAR = 1234
        // Test Mission File
        ServerHost   = localhost
        ServerPort   = 9000
        Community    = alpha

        ${TEST}      = 12
        MOOSTimeWarp = 1


        // MIT Sailing Pavilion
        LatOrigin  = 42.35846207515723
        LongOrigin = -71.08774014042629

        //------------------------------------------
        // Antler configuration  block
        ProcessConfig = ANTLER
        {
          MSBetweenLaunches = 200
          ExecutablePath = system // System path
          Run = MOOSDB          @ NewConsole = false
          Run = pLogger         @ NewConsole = true
          Run = uSimMarine      @ NewConsole = false
          Run = pMarinePID      @ NewConsole = false
          Run = pHelmIvP        @ NewConsole = true, ExtraProcessParams=HParams
          Run = pMarineViewer	@ NewConsole = false
          Run = uProcessWatch	@ NewConsole = false
          Run = pNodeReporter	@ NewConsole = false
          Run = uMemWatch       @ NewConsole = false
          Run = pXRelay @ NewConsole = true ~ pXRelay_PEARS

          // Helm Params
          HParams=--alias=pHelmIvP_Standby
        }
        define: MY_VAR = "this is a test"
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

        let mut lexer = Lexer::new(input);

        while let Some(Ok((_, token, _))) = lexer.next() {
            println!("Parser Token: {:?}", token);
        }

        lexer = Lexer::new(input);
        let mut state = State::default();
        let result = moos::LinesParser::new().parse(&mut state, input, lexer);
        println!("Result: {:?}", result);
        println!("\nErrors: {:?}", state.errors);
        println!("\nDefines: {:?}", state.defines);

        //assert!(errors.is_empty())
    }
}
