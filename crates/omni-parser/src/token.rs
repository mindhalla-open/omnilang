//! Token types for the OmniLang lexer.

use crate::Span;

/// A single token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// The raw source text of this token.
    pub text: String,
}

/// All possible token types in OmniLang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // ── Layout ────────────────────────────────────────
    /// Indentation increase
    Indent,
    /// Indentation decrease
    Dedent,
    /// End of logical line / statement
    Newline,

    // ── Literals ──────────────────────────────────────
    /// Integer literal: `42`, `1_000`
    IntLiteral,
    /// Float literal: `3.14`, `0.99`
    FloatLiteral,
    /// String literal: `"hello world"`
    StringLiteral,
    /// Duration literal: `5min`, `200ms`, `30s`, `2h`, `14days`
    DurationLiteral,
    /// Money literal: `$0.10`, `$1.00`
    MoneyLiteral,
    /// Percentage literal: `85%`, `99.9%`
    PercentageLiteral,

    // ── Keywords ──────────────────────────────────────
    // Module system
    KwModule,
    KwUse,
    KwAs,
    KwVersion,
    KwTarget,
    KwExport,
    KwPrivate,
    KwMixin,
    KwApply,
    KwConstraints,
    KwTargetDependencies,

    // Type declarations
    KwType,
    KwStruct,
    KwEnum,

    // Block types
    KwService,
    KwComponent,
    KwPipeline,
    KwWorkflow,
    KwAgent,
    KwSchema,
    KwPolicy,
    KwContract,
    KwConstraint,
    KwBudget,
    KwEvidence,
    KwOperation,
    KwMetrics,
    KwCounter,
    KwGauge,
    KwHistogram,

    // Service internals
    KwGoal,
    KwInputs,
    KwOutputs,
    KwPreconditions,
    KwPostconditions,
    KwInvariants,
    KwErrors,
    KwDependsOn,
    KwDependencies,
    KwPolicies,

    // Component internals
    KwProps,
    KwState,
    KwEvents,
    KwSlots,

    // Pipeline internals
    KwSource,
    KwStages,
    KwSink,

    // Workflow internals
    KwStates,
    KwTransitions,
    KwTriggers,

    // Policy internals
    KwRules,
    KwSchedule,

    // Schema internals
    KwEntity,
    KwAction,
    KwRule,
    KwRelations,
    KwIndexes,

    // Agent boundaries
    KwCannot,
    KwMust,

    // Other section keywords
    KwStyleGuide,
    KwVisualSpec,
    KwDescription,
    KwScope,
    KwCapabilities,
    KwBoundaries,
    KwTools,
    KwModel,

    // Test keywords
    KwTests,
    KwScenario,
    KwProperty,
    KwGiven,
    KwWhen,
    KwExpect,
    KwExpectError,
    KwAssert,
    KwForall,

    // Logic keywords
    KwIf,
    KwElse,
    KwIn,
    KwNot,
    KwAnd,
    KwOr,

    // Built-in type-like keywords
    KwOption,
    KwResult,
    KwTrue,
    KwFalse,
    KwNull,
    KwNone,
    KwSome,
    KwOk,
    KwErr,
    KwOld,
    KwSelfKw,

    // Other keywords
    KwFactory,
    KwDefine,
    KwVisual,
    KwBenchmark,
    KwChaos,
    KwSecurity,

    // ── Operators ─────────────────────────────────────
    /// `==`
    EqEq,
    /// `!=`
    BangEq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    LtEq,
    /// `>=`
    GtEq,
    /// `&&`
    AmpAmp,
    /// `||`
    PipePipe,
    /// `!`
    Bang,
    /// `..`
    DotDot,
    /// `..<`
    DotDotLt,
    /// `->`
    Arrow,
    /// `|`
    Pipe,
    /// `&`
    Amp,
    /// `?`
    Question,
    /// `@`
    At,
    /// `*`
    Star,

    // ── Delimiters ────────────────────────────────────
    /// `{`
    BraceOpen,
    /// `}`
    BraceClose,
    /// `(`
    ParenOpen,
    /// `)`
    ParenClose,
    /// `[`
    BracketOpen,
    /// `]`
    BracketClose,

    // ── Punctuation ───────────────────────────────────
    /// `:`
    Colon,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `=`
    Eq,
    /// `-`
    Minus,
    /// `+`
    Plus,
    /// `/`
    Slash,

    // ── Identifiers & Comments ────────────────────────
    /// Any identifier: `checkout`, `OrderStatus`, `MAX_RETRIES`
    Ident,
    /// Line comment: `// ...`
    LineComment,
    /// Doc comment: `/// ...`
    DocComment,
    /// Block comment: `/* ... */`
    BlockComment,

    // ── Special ───────────────────────────────────────
    /// End of file
    Eof,
    /// Erroneous token (for error recovery)
    Error,
}

impl TokenKind {
    /// Try to match an identifier string to a keyword.
    pub fn from_keyword(word: &str) -> Option<TokenKind> {
        match word {
            // Module system
            "module" => Some(TokenKind::KwModule),
            "use" => Some(TokenKind::KwUse),
            "as" => Some(TokenKind::KwAs),
            "version" => Some(TokenKind::KwVersion),
            "target" => Some(TokenKind::KwTarget),
            "target_dependencies" => Some(TokenKind::KwTargetDependencies),
            "export" => Some(TokenKind::KwExport),
            "private" => Some(TokenKind::KwPrivate),
            "mixin" => Some(TokenKind::KwMixin),
            "apply" => Some(TokenKind::KwApply),
            "constraints" => Some(TokenKind::KwConstraints),

            // Type declarations
            "type" => Some(TokenKind::KwType),
            "struct" => Some(TokenKind::KwStruct),
            "enum" => Some(TokenKind::KwEnum),

            // Block types
            "service" => Some(TokenKind::KwService),
            // "component" => Some(TokenKind::KwComponent),
            // "pipeline" => Some(TokenKind::KwPipeline),
            "workflow" => Some(TokenKind::KwWorkflow),
            "orchestrator" => Some(TokenKind::KwWorkflow),
            "agent" => Some(TokenKind::KwAgent),
            "schema" => Some(TokenKind::KwSchema),
            "policy" => Some(TokenKind::KwPolicy),
            // "contract" => Some(TokenKind::KwContract),
            "constraint" => Some(TokenKind::KwConstraint),
            "budget" => Some(TokenKind::KwBudget),
            "evidence" => Some(TokenKind::KwEvidence),
            "operation" => Some(TokenKind::KwOperation),
            "metrics" => Some(TokenKind::KwMetrics),
            "counter" => Some(TokenKind::KwCounter),
            "gauge" => Some(TokenKind::KwGauge),
            "histogram" => Some(TokenKind::KwHistogram),

            // Service internals
            "goal" => Some(TokenKind::KwGoal),
            "inputs" => Some(TokenKind::KwInputs),
            "outputs" => Some(TokenKind::KwOutputs),
            "preconditions" => Some(TokenKind::KwPreconditions),
            "postconditions" => Some(TokenKind::KwPostconditions),
            "invariants" => Some(TokenKind::KwInvariants),
            "errors" => Some(TokenKind::KwErrors),
            "depends_on" => Some(TokenKind::KwDependsOn),
            "dependencies" => Some(TokenKind::KwDependencies),
            "policies" => Some(TokenKind::KwPolicies),

            // Component internals
            // "props" => Some(TokenKind::KwProps),
            // "state" => Some(TokenKind::KwState),
            // "events" => Some(TokenKind::KwEvents),
            // "slots" => Some(TokenKind::KwSlots),

            // Pipeline internals
            // "source" => Some(TokenKind::KwSource),
            // "stages" => Some(TokenKind::KwStages),
            // "sink" => Some(TokenKind::KwSink),

            // Workflow internals
            "states" => Some(TokenKind::KwStates),
            "transitions" => Some(TokenKind::KwTransitions),
            // "triggers" => Some(TokenKind::KwTriggers),

            // Policy internals
            // "rules" => Some(TokenKind::KwRules),
            // "schedule" => Some(TokenKind::KwSchedule),

            // Schema & Agent boundary internals
            "entity" => Some(TokenKind::KwEntity),
            "action" => Some(TokenKind::KwAction),
            "rule" => Some(TokenKind::KwRule),
            "relations" => Some(TokenKind::KwRelations),
            "indexes" => Some(TokenKind::KwIndexes),
            "cannot" => Some(TokenKind::KwCannot),
            "must" => Some(TokenKind::KwMust),

            // Other section keywords
            // "style_guide" => Some(TokenKind::KwStyleGuide),
            // "visual_spec" => Some(TokenKind::KwVisualSpec),
            // "description" => Some(TokenKind::KwDescription),
            // "scope" => Some(TokenKind::KwScope),
            "capabilities" => Some(TokenKind::KwCapabilities),
            "boundaries" => Some(TokenKind::KwBoundaries),
            "tools" => Some(TokenKind::KwTools),
            "model" => Some(TokenKind::KwModel),

            // Test keywords
            "tests" => Some(TokenKind::KwTests),
            "scenario" => Some(TokenKind::KwScenario),
            "property" => Some(TokenKind::KwProperty),
            "given" => Some(TokenKind::KwGiven),
            "when" => Some(TokenKind::KwWhen),
            "expect" => Some(TokenKind::KwExpect),
            "expect_error" => Some(TokenKind::KwExpectError),
            "assert" => Some(TokenKind::KwAssert),
            "forall" => Some(TokenKind::KwForall),

            // Logic keywords
            "if" => Some(TokenKind::KwIf),
            "else" => Some(TokenKind::KwElse),
            "in" => Some(TokenKind::KwIn),
            "not" => Some(TokenKind::KwNot),
            "and" => Some(TokenKind::KwAnd),
            "or" => Some(TokenKind::KwOr),

            // Built-in type-like keywords
            "option" => Some(TokenKind::KwOption),
            "result" => Some(TokenKind::KwResult),
            "true" => Some(TokenKind::KwTrue),
            "false" => Some(TokenKind::KwFalse),
            "null" => Some(TokenKind::KwNull),
            "none" => Some(TokenKind::KwNone),
            "some" => Some(TokenKind::KwSome),
            "ok" => Some(TokenKind::KwOk),
            "err" => Some(TokenKind::KwErr),
            "old" => Some(TokenKind::KwOld),
            "self" => Some(TokenKind::KwSelfKw),

            // Other keywords
            // "factory" => Some(TokenKind::KwFactory),
            // "define" => Some(TokenKind::KwDefine),
            // "visual" => Some(TokenKind::KwVisual),
            // "benchmark" => Some(TokenKind::KwBenchmark),
            // "chaos" => Some(TokenKind::KwChaos),
            // "security" => Some(TokenKind::KwSecurity),
            _ => None,
        }
    }

    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::KwModule
                | TokenKind::KwUse
                | TokenKind::KwAs
                | TokenKind::KwVersion
                | TokenKind::KwTarget
                | TokenKind::KwTargetDependencies
                | TokenKind::KwExport
                | TokenKind::KwPrivate
                | TokenKind::KwMixin
                | TokenKind::KwApply
                | TokenKind::KwConstraints
                | TokenKind::KwType
                | TokenKind::KwStruct
                | TokenKind::KwEnum
                | TokenKind::KwService
                // | TokenKind::KwComponent
                // | TokenKind::KwPipeline
                | TokenKind::KwWorkflow
                | TokenKind::KwAgent
                | TokenKind::KwSchema
                | TokenKind::KwPolicy
                // | TokenKind::KwContract
                | TokenKind::KwConstraint
                | TokenKind::KwBudget
                | TokenKind::KwEvidence
                | TokenKind::KwOperation
                | TokenKind::KwMetrics
                | TokenKind::KwCounter
                | TokenKind::KwGauge
                | TokenKind::KwHistogram
                | TokenKind::KwGoal
                | TokenKind::KwInputs
                | TokenKind::KwOutputs
                | TokenKind::KwPreconditions
                | TokenKind::KwPostconditions
                | TokenKind::KwInvariants
                | TokenKind::KwErrors
                | TokenKind::KwDependsOn
                | TokenKind::KwDependencies
                | TokenKind::KwPolicies
                // | TokenKind::KwProps
                // | TokenKind::KwState
                // | TokenKind::KwEvents
                // | TokenKind::KwSlots
                // | TokenKind::KwSource
                // | TokenKind::KwStages
                // | TokenKind::KwSink
                | TokenKind::KwStates
                | TokenKind::KwTransitions
                // | TokenKind::KwTriggers
                // | TokenKind::KwRules
                // | TokenKind::KwSchedule
                | TokenKind::KwEntity
                | TokenKind::KwAction
                | TokenKind::KwRule
                | TokenKind::KwRelations
                | TokenKind::KwIndexes
                | TokenKind::KwCannot
                | TokenKind::KwMust

                // | TokenKind::KwStyleGuide
                // | TokenKind::KwVisualSpec
                // | TokenKind::KwDescription
                // | TokenKind::KwScope
                | TokenKind::KwCapabilities
                | TokenKind::KwBoundaries
                | TokenKind::KwTools
                | TokenKind::KwModel
                | TokenKind::KwTests
                | TokenKind::KwScenario
                | TokenKind::KwProperty
                | TokenKind::KwGiven
                | TokenKind::KwWhen
                | TokenKind::KwExpect
                | TokenKind::KwExpectError
                | TokenKind::KwAssert
                | TokenKind::KwForall
                | TokenKind::KwIf
                | TokenKind::KwElse
                | TokenKind::KwIn
                | TokenKind::KwNot
                | TokenKind::KwAnd
                | TokenKind::KwOr
                | TokenKind::KwOption
                | TokenKind::KwResult
                | TokenKind::KwTrue
                | TokenKind::KwFalse
                | TokenKind::KwNull
                | TokenKind::KwNone
                | TokenKind::KwSome
                | TokenKind::KwOk
                | TokenKind::KwErr
                | TokenKind::KwOld
                | TokenKind::KwSelfKw // | TokenKind::KwFactory
                                      // | TokenKind::KwDefine
                                      // | TokenKind::KwVisual
                                      // | TokenKind::KwBenchmark
                                      // | TokenKind::KwChaos
                                      // | TokenKind::KwSecurity
        )
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Indent => write!(f, "indent"),
            TokenKind::Dedent => write!(f, "dedent"),
            TokenKind::Newline => write!(f, "newline"),
            TokenKind::IntLiteral => write!(f, "integer"),
            TokenKind::FloatLiteral => write!(f, "float"),
            TokenKind::StringLiteral => write!(f, "string"),
            TokenKind::DurationLiteral => write!(f, "duration"),
            TokenKind::MoneyLiteral => write!(f, "money"),
            TokenKind::PercentageLiteral => write!(f, "percentage"),
            TokenKind::Ident => write!(f, "identifier"),
            TokenKind::Eof => write!(f, "end of file"),
            TokenKind::Error => write!(f, "error"),
            TokenKind::LineComment => write!(f, "comment"),
            TokenKind::DocComment => write!(f, "doc comment"),
            TokenKind::BlockComment => write!(f, "block comment"),
            TokenKind::EqEq => write!(f, "'=='"),
            TokenKind::BangEq => write!(f, "'!='"),
            TokenKind::Lt => write!(f, "'<'"),
            TokenKind::Gt => write!(f, "'>'"),
            TokenKind::LtEq => write!(f, "'<='"),
            TokenKind::GtEq => write!(f, "'>='"),
            TokenKind::AmpAmp => write!(f, "'&&'"),
            TokenKind::PipePipe => write!(f, "'||'"),
            TokenKind::Bang => write!(f, "'!'"),
            TokenKind::DotDot => write!(f, "'..'"),
            TokenKind::DotDotLt => write!(f, "'..<'"),
            TokenKind::Arrow => write!(f, "'->'"),
            TokenKind::Pipe => write!(f, "'|'"),
            TokenKind::Amp => write!(f, "'&'"),
            TokenKind::Question => write!(f, "'?'"),
            TokenKind::At => write!(f, "'@'"),
            TokenKind::Star => write!(f, "'*'"),
            TokenKind::BraceOpen => write!(f, "'{{'"),
            TokenKind::BraceClose => write!(f, "'}}'"),
            TokenKind::ParenOpen => write!(f, "'('"),
            TokenKind::ParenClose => write!(f, "')'"),
            TokenKind::BracketOpen => write!(f, "'['"),
            TokenKind::BracketClose => write!(f, "']'"),
            TokenKind::Colon => write!(f, "':'"),
            TokenKind::Comma => write!(f, "','"),
            TokenKind::Dot => write!(f, "'.'"),
            TokenKind::Eq => write!(f, "'='"),
            TokenKind::Minus => write!(f, "'-'"),
            TokenKind::Plus => write!(f, "'+'"),
            TokenKind::Slash => write!(f, "'/'"),
            _ if self.is_keyword() => write!(f, "keyword"),
            _ => write!(f, "{:?}", self),
        }
    }
}
