//! Programs as data. GRAMMAR.md is the specification; this crate is its implementation.
//!
//! The size counter is not a convenience: GRAMMAR 6 states the project's prior as the node
//! lengths of the reference programs, and those were hand estimates flagged +/-20% until this
//! crate could measure them.

pub mod ast;
pub mod reference;
pub mod mutate;
pub mod sexp;
pub mod typecheck;

pub use ast::{ArithOp, FieldId, Func, Lineage, Node, OutcomeLit, PredId, Program, Rel, Ty};
