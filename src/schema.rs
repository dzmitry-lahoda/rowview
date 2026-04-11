//! Parsed and validated intermediate representation for code generation.

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

pub(super) struct RowsBuildPlan {
    pub(super) args: RowsArgs,
    pub(super) module: RowsModule,
    pub(super) rowsets: Vec<RowsetBuildPlan>,
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
    pub(super) agg_convert_into: bool,
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

pub(super) struct RowsetBuildPlan {
    pub(super) rowset_index: usize,
    pub(super) nested_axis: Option<NestedAxisSpec>,
    pub(super) index_join_len_asserts: Vec<IndexJoinLenAssertPlan>,
    pub(super) zip_join_key_asserts: Vec<ZipJoinKeyAssertPlan>,
    pub(super) row_joins: Vec<RowJoinPlan>,
}

pub(super) struct IndexJoinLenAssertPlan {
    pub(super) source: Expr,
}

pub(super) struct ZipJoinKeyAssertPlan {
    pub(super) source: Expr,
    pub(super) condition: Expr,
    pub(super) alias: Option<Ident>,
}

pub(super) struct RowJoinPlan {
    pub(super) join_index: usize,
    pub(super) binding: Ident,
    pub(super) join_axis: Expr,
}

pub(super) struct NestedAxisSpec {
    pub(super) parent: Expr,
    pub(super) child: Member,
}
