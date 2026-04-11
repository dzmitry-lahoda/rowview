//! Specialization of matched Rust expression patterns.
//! Forms a pure Rust-based eDSL and acts as documentation for all possible features.

use crate::docs::{FieldKind, FieldMode};
use syn::{Attribute, Expr, Ident, ItemUse, Member, Visibility};

pub(super) struct RowsArgs {
    pub(super) root: Ident,
}

pub(super) struct RowsModule {
    pub(super) vis: Visibility,
    pub(super) name: Ident,
    pub(super) imports: Vec<ItemUse>,
    pub(super) rowsets: Vec<RowsetSpec>,
}

pub(super) struct RowsetSpec {
    pub(super) attrs: Vec<Attribute>,
    pub(super) joins: Vec<JoinOptionSpec>,
    pub(super) rowset_name: Ident,
    pub(super) axis: Expr,
    pub(super) struct_name: Ident,
    pub(super) fields: Vec<FieldSpec>,
}

pub(super) struct FieldSpec {
    pub(super) kind: FieldKind,
    pub(super) mode: FieldMode,
    pub(super) name: Ident,
    pub(super) ty: syn::Type,
    pub(super) expr: Expr,
    pub(super) join: Option<JoinOptionSpec>,
}

pub(super) struct IncrementExpr {
    pub(super) expr: Expr,
}

pub(super) struct JoinOptionSpec {
    pub(super) source: Option<Expr>,
    pub(super) alias: Option<Ident>,
    pub(super) condition: Option<Expr>,
    pub(super) by_index: bool,
    pub(super) required: bool,
    pub(super) zipped: bool,
    pub(super) value: Option<Expr>,
}

pub(super) struct NestedAxisSpec {
    pub(super) parent: Expr,
    pub(super) child: Member,
}
