/*
 * namespace	For identifiers that declare or reference a namespace, module, or package.
 * class	For identifiers that declare or reference a class type.
 * enum	For identifiers that declare or reference an enumeration type.
 * interface	For identifiers that declare or reference an interface type.
 * struct	For identifiers that declare or reference a struct type.
 * typeParameter	For identifiers that declare or reference a type parameter.
 * type	For identifiers that declare or reference a type that is not covered above.
 * parameter	For identifiers that declare or reference a function or method parameters.
 * variable	For identifiers that declare or reference a local or global variable.
 * property	For identifiers that declare or reference a member property, member field, or member variable.
 * enumMember	For identifiers that declare or reference an enumeration property, constant, or member.
 * decorator	For identifiers that declare or reference decorators and annotations.
 * event	For identifiers that declare an event property.
 * function	For identifiers that declare a function.
 * method	For identifiers that declare a member function or method.
 * macro	For identifiers that declare a macro.
 * label	For identifiers that declare a label.
 * comment	For tokens that represent a comment.
 * string	For tokens that represent a string literal.
 * keyword	For tokens that represent a language keyword.
 * number	For tokens that represent a number literal.
 * regexp	For tokens that represent a regular expression literal.
 * operator	For tokens that represent an operator.
 */
pub const TOKEN_TYPES: &'static [&'static str] = &[
    "comment", "keyword", "variable", "string", "number", "macro", "type",
];

/*
* declaration	For declarations of symbols.
* definition	For definitions of symbols, for example, in header files.
* readonly	For readonly variables and member fields (constants).
* static	For class members (static members).
* deprecated	For symbols that should no longer be used.
* abstract	For types and member functions that are abstract.
* async	For functions that are marked async.
* modification	For variable references where the variable is assigned to.
* documentation	For occurrences of symbols in documentation.
* defaultLibrary	For symbols that are part of the standard library.
*/

pub const TOKEN_MODIFIERS: &'static [&'static str] =
    &["declaration", "documentation", "deprecated"];

pub enum TokenTypes {
    /// For tokens that represent a comment.
    Comment = 0,
    /// For tokens that represent a language keyword.
    Keyword,
    /// For identifiers that declare or reference a local or global variable.
    Variable,
    /// For tokens that represent a string literal.
    String,
    /// For tokens that represent a number literal.
    Number,
    /// For identifiers that declare a macro.
    Macro,
    /// For tokens that represent an operator
    Operator,
}

impl Into<u32> for TokenTypes {
    fn into(self) -> u32 {
        self as u32
    }
}

pub enum TokenModifiers {
    /// When no modifiers are needed
    None = 0,
    /// For declarations of symbols.
    Declaration = 0x01,
    /// For occurrences of symbols in documentation.
    Documentation = 0x02,
    /// For symbols that should no longer be used.
    Deprecated = 0x04,
}

impl Into<u32> for TokenModifiers {
    fn into(self) -> u32 {
        self as u32
    }
}

impl core::ops::BitOr for TokenModifiers {
    type Output = u32;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as u32 | rhs as u32
    }
}

impl core::ops::BitOr<TokenModifiers> for u32 {
    type Output = u32;

    fn bitor(self, rhs: TokenModifiers) -> Self::Output {
        self | rhs as u32
    }
}
