//! Abstract Syntax Tree definitions for OmniLang.
//!
//! These types represent the parsed structure of `.omni` source files.
//! The AST is produced by the parser and consumed by the analyzer.

use crate::Span;

/// Visibility of a declaration.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, specta::Type, schemars::JsonSchema,
)]
pub enum Visibility {
    /// Public (default) — visible to importing modules.
    Public,
    /// Private — only visible within the declaring module.
    Private,
}

/// A complete OmniLang source file.
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct SourceFile {
    /// Module declaration (required, first statement).
    pub module: ModuleDecl,
    /// Import statements.
    pub imports: Vec<ImportDecl>,
    /// Exported symbol names (from `export` declarations).
    pub exports: Vec<String>,
    /// Top-level declarations.
    pub declarations: Vec<Declaration>,
}

/// Module declaration: `module acme.payments.checkout`
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ModuleDecl {
    /// Dotted module path, e.g. `["acme", "payments", "checkout"]`.
    pub path: Vec<String>,
    pub span: Span,
}

/// Import declaration: `use std.http.{Request, Response}`
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ImportDecl {
    /// The import path segments.
    pub path: Vec<String>,
    /// Specific items to import, or empty for wildcard `*`.
    pub items: ImportItems,
    /// Import kind: standard, relative, or registry.
    pub kind: ImportKind,
    pub span: Span,
}

/// The kind of import path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub enum ImportKind {
    /// Standard dotted path: `use std.http.Request`
    Standard,
    /// Relative path: `use ./shared/types.*`
    Relative,
    /// Registry import: `use registry://acme/shared@2.0.*`
    Registry {
        registry: String,
        version: Option<String>,
    },
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub enum ImportItems {
    /// `use std.auth.*`
    Wildcard,
    /// `use std.http.{Request, Response}`
    Named(Vec<ImportItem>),
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ImportItem {
    pub name: String,
    /// Optional alias: `use std.money.Money as Currency`
    pub alias: Option<String>,
    pub span: Span,
}

/// A top-level declaration.
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub enum Declaration {
    Type(TypeDecl),
    Service(ServiceDecl),
    Component(ComponentDecl),
    Pipeline(PipelineDecl),
    Workflow(WorkflowDecl),
    Agent(AgentDecl),
    Schema(SchemaDecl),
    Policy(PolicyDecl),
    Constraint(ConstraintDecl),
    Mixin(MixinDecl),
    TargetDependencies(TargetDependenciesDecl),
    Entity(EntityDecl),
    Action(ActionDecl),
    Rule(RuleDecl),
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct RuleDecl {
    pub name: String,
    pub target: String,
    pub condition: Expression,
    pub doc_comment: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ActionDecl {
    pub name: String,
    pub inputs: Vec<Field>,
    pub outputs: Vec<Field>,
    pub preconditions: Vec<Expression>,
    pub postconditions: Vec<Expression>,
    pub doc_comment: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ConstraintDecl {
    pub name: String,
    pub requires: Vec<Constraint>,
    pub verification: Vec<VerificationEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct VerificationEntry {
    pub tool: String,
    pub evidence: String,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Type declarations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<TypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct TypeDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub kind: TypeKind,
    pub visibility: Visibility,
    pub doc_comment: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub enum TypeKind {
    /// `type OrderStatus = enum { Draft, Pending, ... }`
    Enum(EnumType),
    /// `type Customer = struct { id: CustomerId, ... }`
    Struct(StructType),
    /// `type OrderId = String { format: regex("...") }`
    Refined(RefinedType),
    /// `type UserId = UUID` (simple alias)
    Alias(TypeRef),
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct EnumType {
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct EnumVariant {
    pub name: String,
    /// Optional associated data: `Shipped(tracking_id: String)`
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct StructType {
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct Field {
    pub name: String,
    pub ty: TypeRef,
    pub default: Option<Expression>,
    pub doc_comment: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct RefinedType {
    pub base: Option<TypeRef>,
    pub constraints: Vec<TypeConstraint>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct TypeConstraint {
    pub name: String,
    pub value: Expression,
    pub span: Span,
}

/// A reference to a type, e.g. `String`, `List<Order>`, `Option<Int>`.
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct TypeRef {
    pub name: String,
    pub type_args: Vec<TypeRef>,
    pub union_members: Vec<TypeRef>,
    pub intersection_members: Vec<TypeRef>,
    pub span: Span,
}

impl TypeRef {
    pub fn simple(name: String, span: Span) -> Self {
        Self {
            name,
            type_args: Vec::new(),
            union_members: Vec::new(),
            intersection_members: Vec::new(),
            span,
        }
    }
}

// ---------------------------------------------------------------------------
// Service declarations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct DependencyRef {
    pub name: String,
    pub notes: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ServicePolicy {
    pub name: String,
    pub entries: Vec<ConfigEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ServiceDecl {
    pub name: String,
    pub goal: Option<String>,
    /// Per-service target language override (`target rust`). `None` = use the
    /// build-wide target.
    pub target: Option<String>,
    pub constraints: Vec<Constraint>,
    pub depends_on: Vec<String>,
    pub dependencies: Vec<DependencyRef>,
    pub policies: Vec<ServicePolicy>,
    pub operations: Vec<OperationDecl>,
    pub budget: Option<BudgetBlock>,
    pub metrics: Vec<MetricDecl>,
    pub invariants: Vec<Expression>,
    pub applies: Vec<String>,
    pub visibility: Visibility,
    pub doc_comment: Option<String>,
    pub span: Span,
}

/// A reusable mixin block.
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct MixinDecl {
    pub name: String,
    pub constraints: Vec<Constraint>,
    pub postconditions: Vec<Expression>,
    pub tests: Vec<TestBlock>,
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct MetricDecl {
    pub name: String,
    pub kind: MetricKind,
    pub description: Option<String>,
    pub labels: Vec<String>,
    pub buckets: Option<Vec<Expression>>,
    pub span: Span,
}

#[derive(
    Debug, Clone, Copy, serde::Serialize, specta::Type, PartialEq, Eq, schemars::JsonSchema,
)]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct OperationDecl {
    pub name: String,
    pub inputs: Vec<Field>,
    pub outputs: Vec<Field>,
    pub preconditions: Vec<Expression>,
    pub postconditions: Vec<Expression>,
    pub errors: Vec<ErrorDecl>,
    pub constraints: Vec<Constraint>,
    pub tests: Vec<TestBlock>,
    pub doc_comment: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ErrorDecl {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct Constraint {
    pub name: String,
    pub args: Vec<ConstraintArg>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ConstraintArg {
    pub name: Option<String>,
    pub value: Expression,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Test blocks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct TestBlock {
    pub kind: TestKind,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub enum TestKind {
    Scenario {
        name: String,
        given: Vec<Expression>,
        when: Vec<Expression>,
        expect: Vec<Expression>,
        expect_error: Option<String>,
    },
    Property {
        name: String,
        quantifiers: Vec<Quantifier>,
        given: Vec<Expression>,
        when: Vec<Expression>,
        assert: Vec<Expression>,
    },
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct Quantifier {
    pub name: String,
    pub generator: Expression,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Budget blocks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct BudgetBlock {
    pub entries: Vec<BudgetEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct BudgetEntry {
    pub key: String,
    pub value: Expression,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Expressions (simplified for Phase 0)
// ---------------------------------------------------------------------------

/// Expressions used in constraints, pre/postconditions, and test assertions.
#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub enum Expression {
    /// A literal value: `42`, `"hello"`, `true`, `200ms`, `$0.10`
    Literal(Literal),
    /// An identifier: `order_id`, `PaymentStatus`
    Identifier(String, Span),
    /// Binary operation: `a == b`, `balance > 0`
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
        span: Span,
    },
    /// Unary operation: `!deleted`
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
        span: Span,
    },
    /// Function call: `old(balance)`, `sum(items)`
    Call {
        function: String,
        type_args: Option<Vec<TypeRef>>,
        args: Vec<Expression>,
        span: Span,
    },
    /// Field access: `order.status`
    FieldAccess {
        object: Box<Expression>,
        field: String,
        span: Span,
    },
    /// List literal: `[1, 2, 3]`
    List(Vec<Expression>, Span),
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Duration(String),
    Money(String),
    Null,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, specta::Type, schemars::JsonSchema,
)]
pub enum BinaryOperator {
    Eq,       // ==
    NotEq,    // !=
    Lt,       // <
    Gt,       // >
    LtEq,     // <=
    GtEq,     // >=
    And,      // &&
    Or,       // ||
    In,       // in
    NotIn,    // not in
    Range,    // ..
    RangeExc, // ..<
}

#[derive(Debug, Clone, Copy, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub enum UnaryOperator {
    Not, // !
    Neg, // -
}

// ---------------------------------------------------------------------------
// Phase 1: Component, Pipeline, Workflow, Agent, Schema, Policy declarations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ComponentDecl {
    pub name: String,
    pub goal: Option<String>,
    pub props: Vec<Field>,
    pub state: Vec<Field>,
    pub events: Vec<EventDecl>,
    pub slots: Vec<Field>,
    pub constraints: Vec<Constraint>,
    pub style_guide: Option<String>,
    pub visual_spec: Vec<String>,
    pub tests: Vec<TestBlock>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct EventDecl {
    pub name: String,
    pub params: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct PipelineDecl {
    pub name: String,
    pub goal: Option<String>,
    pub source: Vec<ConfigEntry>,
    pub stages: Vec<PipelineStage>,
    pub sink: Vec<ConfigEntry>,
    pub constraints: Vec<Constraint>,
    pub schedule: Option<String>,
    pub tests: Vec<TestBlock>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct ConfigEntry {
    pub key: String,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct PipelineStage {
    pub name: String,
    pub entries: Vec<ConfigEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct WorkflowDecl {
    pub name: String,
    pub goal: Option<String>,
    pub states: Vec<String>,
    pub transitions: Vec<WorkflowTransition>,
    pub constraints: Vec<Constraint>,
    pub tests: Vec<TestBlock>,
    pub dependencies: Vec<DependencyRef>,
    pub policies: Vec<ServicePolicy>,
    pub invariants: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct WorkflowTransition {
    pub from: String,
    pub to: String,
    pub trigger: Option<String>,
    pub timeout: Option<WorkflowTimeout>,
    pub guard: Option<Expression>,
    pub actions: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct WorkflowTimeout {
    pub duration: Expression,
    pub target_state: String,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct AgentDecl {
    pub name: String,
    pub goal: Option<String>,
    pub capabilities: Vec<String>,
    pub boundaries: Vec<AgentBoundary>,
    pub tools: Vec<AgentTool>,
    pub model: Vec<ConfigEntry>,
    pub budget: Option<BudgetBlock>,
    pub tests: Vec<TestBlock>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct AgentBoundary {
    pub kind: BoundaryKind,
    pub expr: Expression,
    pub span: Span,
}

#[derive(
    Debug, Clone, Copy, serde::Serialize, specta::Type, PartialEq, Eq, schemars::JsonSchema,
)]
pub enum BoundaryKind {
    Must,
    Cannot,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct AgentTool {
    pub name: String,
    pub inputs: Vec<Field>,
    pub outputs: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct SchemaDecl {
    pub name: String,
    pub goal: Option<String>,
    pub target: Option<String>,
    pub entities: Vec<EntityDecl>,
    pub relations: Vec<RelationDecl>,
    pub indexes: Vec<IndexDecl>,
    pub constraints: Vec<Constraint>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct EntityDecl {
    pub name: String,
    pub fields: Vec<EntityField>,
    pub doc_comment: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct EntityField {
    pub name: String,
    pub ty: TypeRef,
    pub default: Option<Expression>,
    pub decorators: Vec<Decorator>,
    pub doc_comment: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct Decorator {
    pub name: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct RelationDecl {
    pub lhs: String,
    pub rel_type: String,
    pub rhs: String,
    pub args: Vec<ConfigEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct IndexDecl {
    pub entity: String,
    pub fields: Vec<String>,
    pub r#where: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct PolicyDecl {
    pub name: String,
    pub description: Option<String>,
    pub scope: Option<String>,
    pub rules: Vec<PolicyRule>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct PolicyRule {
    pub condition: String,
    pub clauses: Vec<PolicyClause>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub enum PolicyClause {
    Simple(String, Span),
    Action {
        verb: String,
        value: Expression,
        span: Span,
    },
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    specta::Type,
    schemars::JsonSchema,
    serde::Deserialize,
)]
pub enum TrustLevel {
    Speculative,
    Low,
    Medium,
    High,
    Proven,
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct TargetDependenciesDecl {
    pub entries: Vec<TargetDependencyEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct TargetDependencyEntry {
    pub target: String,
    pub packages: Vec<DependencyPackage>,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type, schemars::JsonSchema)]
pub struct DependencyPackage {
    pub name: String,
    pub version: String,
    pub span: Span,
}
