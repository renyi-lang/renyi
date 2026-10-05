//! The abstract syntax tree, one node per construct of the syntax sketch.
//! Every node carries the span of its source text so that diagnostics and the
//! formatter can point back at it.

use crate::span::Span;

/// A lowercase identifier or member name.
#[derive(Clone, Debug, PartialEq)]
pub struct Name {
    pub text: String,
    pub span: Span,
}

/// A PascalCase type, ability or variant name.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeName {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub name: Vec<Name>,
    pub docs: Docs,
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
    /// Comment spans in source order, kept for the formatter.
    pub comments: Vec<Span>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Import {
    pub path: Vec<Name>,
    pub alias: Option<Name>,
    pub exposing: Vec<TypeName>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Function(Function),
    Type(TypeDef),
    Ability(AbilityDecl),
    Implementation(AbilityImpl),
    Constant(Constant),
    Test(Test),
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Function(item) => item.span,
            Item::Type(item) => item.span,
            Item::Ability(item) => item.span,
            Item::Implementation(item) => item.span,
            Item::Constant(item) => item.span,
            Item::Test(item) => item.span,
        }
    }
}

/// Documentation clauses shared by modules, functions, types and constants.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Docs {
    pub purpose: Option<String>,
    pub tags: Vec<String>,
    pub see_also: Vec<String>,
    pub deprecated: Option<String>,
    pub expose_as_tool: bool,
    pub examples: Vec<Example>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Example {
    pub expression: Expr,
    pub outcome: ExampleOutcome,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExampleOutcome {
    Is(Expr),
    FailsWith(Pattern),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub public: bool,
    pub name: Name,
    pub params: Vec<Param>,
    pub returns: Option<Type>,
    pub fails: Vec<Type>,
    pub needs: Vec<Capability>,
    pub type_params: Option<ForAny>,
    pub docs: Docs,
    /// `None` inside an ability declaration, where only the signature is given.
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: Name,
    /// `None` for the bare `self` of an ability function.
    pub ty: Option<Type>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Capability {
    pub path: Vec<Name>,
    pub scope: Option<String>,
    pub span: Span,
}

/// `for any T, U where T can Compare and U can Hash`.
#[derive(Clone, Debug, PartialEq)]
pub struct ForAny {
    pub params: Vec<TypeName>,
    pub constraints: Vec<Constraint>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constraint {
    pub param: TypeName,
    pub ability: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Named {
        name: TypeName,
        args: Vec<Type>,
        span: Span,
    },
    Maybe(Box<Type>, Span),
    Function {
        params: Vec<Type>,
        returns: Option<Box<Type>>,
        fails: Vec<Type>,
        needs: Vec<Capability>,
        span: Span,
    },
}

impl Type {
    pub fn span(&self) -> Span {
        match self {
            Type::Named { span, .. } | Type::Maybe(_, span) | Type::Function { span, .. } => *span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDef {
    pub public: bool,
    pub name: TypeName,
    pub type_params: Vec<TypeName>,
    pub kind: TypeKind,
    pub docs: Docs,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    Record {
        fields: Vec<Field>,
        derives: Vec<Derive>,
    },
    Sum {
        variants: Vec<Variant>,
        derives: Vec<Derive>,
    },
    /// `type Email is Text where value.matches(...)`.
    Subtype {
        base: Type,
        refinement: Option<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub name: Name,
    pub ty: Type,
    pub refinement: Option<Expr>,
    /// `as "type"`: the name used by JSON and database decoders.
    pub external_name: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub name: TypeName,
    pub fields: Vec<Field>,
    pub span: Span,
}

/// `can Compare by name, age`.
#[derive(Clone, Debug, PartialEq)]
pub struct Derive {
    pub ability: TypeName,
    pub by: Vec<Name>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbilityDecl {
    pub public: bool,
    pub name: TypeName,
    pub type_params: Vec<TypeName>,
    /// `ability Printable where self can ToText`.
    pub requirements: Vec<Type>,
    pub docs: Docs,
    pub functions: Vec<Function>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbilityImpl {
    pub ability: Type,
    pub target: Type,
    pub type_params: Option<ForAny>,
    pub functions: Vec<Function>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constant {
    pub public: bool,
    pub name: Name,
    pub ty: Type,
    pub value: Expr,
    pub docs: Docs,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Test {
    pub name: String,
    pub needs: Vec<Capability>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Let {
        mutable: bool,
        name: Name,
        ty: Option<Type>,
        value: Expr,
    },
    Set {
        name: Name,
        value: Expr,
    },
    If {
        branches: Vec<(Expr, Block)>,
        otherwise: Option<Block>,
    },
    Match {
        subject: Expr,
        arms: Vec<MatchArm<Block>>,
        otherwise: Option<Block>,
    },
    ForEach {
        bindings: Vec<Name>,
        source: Expr,
        filter: Option<Expr>,
        order: Option<Ordering>,
        body: Block,
    },
    While {
        condition: Expr,
        body: Block,
    },
    RunConcurrently {
        within: Option<Expr>,
        body: Block,
    },
    Return(Option<Expr>),
    Fail(Option<Expr>),
    Crash(Expr),
    Break,
    Continue,
    Ignore(Expr),
    Check(Expr),
    /// A call whose result is not used, or any other expression statement.
    Expression(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchArm<Body> {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Body,
    pub span: Span,
}

/// `sorted by key [descending]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Ordering {
    pub key: Expr,
    pub descending: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    /// `Circle(radius)`, `Circle(radius: outer)`, `Circle(radius: 2.5)`, or a bare `Circle`.
    Variant {
        name: TypeName,
        fields: Vec<FieldPattern>,
        span: Span,
    },
    Literal(Expr),
    Nothing(Span),
    Some(Box<Pattern>, Span),
    Success(Box<Pattern>, Span),
    Failure(Box<Pattern>, Span),
    Binding(Name),
    /// `error: HttpError`.
    Typed {
        name: Name,
        ty: Type,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Variant { span, .. }
            | Pattern::Nothing(span)
            | Pattern::Some(_, span)
            | Pattern::Success(_, span)
            | Pattern::Failure(_, span)
            | Pattern::Typed { span, .. } => *span,
            Pattern::Literal(expr) => expr.span,
            Pattern::Binding(name) => name.span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldPattern {
    pub field: Name,
    /// `None` for a punned field, which binds the field name itself.
    pub pattern: Option<Pattern>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextPiece {
    Text(String),
    Hole(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Integer(String),
    Decimal(String),
    Text {
        pieces: Vec<TextPiece>,
        block: bool,
    },
    RawText(String),
    Boolean(bool),
    Nothing,
    SelfValue,
    Name(Name),
    /// A bare variant or type used as a value: `Point`, `Red`.
    TypeName(TypeName),
    Member {
        base: Box<Expr>,
        name: Name,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    Construct {
        name: TypeName,
        args: Vec<Arg>,
    },
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Range {
        from: Box<Expr>,
        to: Box<Expr>,
        by: Option<Box<Expr>>,
    },
    Not(Box<Expr>),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    With {
        base: Box<Expr>,
        updates: Vec<Arg>,
    },
    Otherwise {
        value: Box<Expr>,
        fallback: Box<Outcome>,
    },
    If {
        branches: Vec<(Expr, Outcome)>,
        otherwise: Box<Outcome>,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm<Outcome>>,
        otherwise: Option<Box<Outcome>>,
    },
    Query(Box<Query>),
    Paren(Box<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub name: Option<Name>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Power,
    Is,
    IsNot,
    IsLessThan,
    IsAtMost,
    IsGreaterThan,
    IsAtLeast,
    And,
    Or,
}

impl BinaryOp {
    pub fn spelling(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Remainder => "remainder",
            BinaryOp::Power => "power",
            BinaryOp::Is => "is",
            BinaryOp::IsNot => "is not",
            BinaryOp::IsLessThan => "is less than",
            BinaryOp::IsAtMost => "is at most",
            BinaryOp::IsGreaterThan => "is greater than",
            BinaryOp::IsAtLeast => "is at least",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
        }
    }
}

/// What a branch or an `otherwise` produces: a value, or a way out.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Value(Expr),
    Fail(Option<Expr>, Span),
    Return(Option<Expr>, Span),
    Crash(Expr, Span),
    Break(Span),
    Continue(Span),
}

impl Outcome {
    pub fn span(&self) -> Span {
        match self {
            Outcome::Value(expr) => expr.span,
            Outcome::Fail(_, span)
            | Outcome::Return(_, span)
            | Outcome::Crash(_, span)
            | Outcome::Break(span)
            | Outcome::Continue(span) => *span,
        }
    }
}

/// `for each x in xs where ... sorted by ... collect ...`.
#[derive(Clone, Debug, PartialEq)]
pub struct Query {
    pub sources: Vec<QuerySource>,
    pub concurrently: bool,
    pub within: Option<Expr>,
    pub filter: Option<Expr>,
    pub order: Option<Ordering>,
    pub group_by: Option<Expr>,
    pub terminal: QueryTerminal,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuerySource {
    pub bindings: Vec<Name>,
    pub source: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QueryTerminal {
    Collect(Expr),
    Sum(Expr),
    Count,
    First,
    Any(Expr),
    All(Expr),
    /// `group by key` with no `collect`: groups of whole items.
    None,
}
