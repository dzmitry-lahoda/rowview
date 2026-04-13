//! Parsed and validated schema representation for code generation.

use crate::docs::{FieldKind, FieldMode};
use syn::{Attribute, Expr, Ident, ItemUse, Member, Visibility};

pub(super) struct RowViewArgs {
    pub(super) root: Ident,
}

pub(super) struct SchemaModule {
    pub(super) vis: Visibility,
    pub(super) name: Ident,
    pub(super) imports: Vec<ItemUse>,
    pub(super) relations: Vec<RelationSchema>,
}

pub(super) struct DatabaseBuildPlan {
    pub(super) args: RowViewArgs,
    pub(super) module: SchemaModule,
    pub(super) relations: Vec<RelationBuildPlan>,
}

pub(super) struct RelationSchema {
    pub(super) rust_attributes: Vec<Attribute>,
    pub(super) joins: Vec<JoinSpec>,
    pub(super) bindings: Vec<BindingSpec>,
    pub(super) relation_name: Ident,
    pub(super) generator: RelationGenerator,
    pub(super) struct_name: Ident,
    pub(super) attributes: Vec<AttributeSpec>,
}

pub(super) struct AttributeSpec {
    pub(super) kind: FieldKind,
    pub(super) mode: FieldMode,
    pub(super) name: Ident,
    pub(super) ty: syn::Type,
    pub(super) expr: Expr,
    pub(super) agg_convert_into: bool,
    pub(super) join: Option<JoinSpec>,
}

pub(super) struct IncrementExpr {
    pub(super) expr: Expr,
}

pub(super) struct JoinSpec {
    pub(super) source: Option<Expr>,
    pub(super) alias: Option<Ident>,
    pub(super) condition: Option<Expr>,
    pub(super) lookup: JoinLookup,
    pub(super) miss: JoinMiss,
    pub(super) value: Option<Expr>,
}

pub(super) enum RelationGenerator {
    Axis(Expr),
    Support(SupportSpec),
}

#[derive(Clone)]
pub(super) struct SupportSpec {
    pub(super) sources: Vec<SupportSourceSpec>,
}

#[derive(Clone)]
pub(super) struct SupportSourceSpec {
    pub(super) source: Expr,
    pub(super) key: Expr,
}

pub(super) struct BindingSpec {
    pub(super) source: Expr,
    pub(super) alias: Ident,
    pub(super) lookup: BindingLookup,
    pub(super) filter: Option<BindingFilter>,
}

pub(super) enum BindingLookup {
    Key { expr: Expr },
}

pub(super) enum BindingFilter {
    Some(Ident),
    Any(Vec<BindingFilter>),
    All(Vec<BindingFilter>),
    Not(Box<BindingFilter>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum JoinLookup {
    Predicate,
    Index,
    Zip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum JoinMiss {
    ProjectNone,
    Panic,
    SkipRow,
}

impl JoinSpec {
    pub(super) fn is_index(&self) -> bool {
        matches!(self.lookup, JoinLookup::Index)
    }

    pub(super) fn is_zip(&self) -> bool {
        matches!(self.lookup, JoinLookup::Zip)
    }

    pub(super) fn panics_on_miss(&self) -> bool {
        matches!(self.miss, JoinMiss::Panic)
    }

    pub(super) fn skips_row_on_miss(&self) -> bool {
        matches!(self.miss, JoinMiss::SkipRow)
    }
}

pub(super) struct RelationBuildPlan {
    pub(super) relation_index: usize,
    pub(super) row_existence: RowExistencePlan,
    pub(super) support_bindings: Vec<SupportBindingPlan>,
    pub(super) index_join_len_asserts: Vec<IndexJoinCardinalityPlan>,
    pub(super) zip_join_key_asserts: Vec<ZipJoinCoveragePlan>,
    pub(super) row_joins: Vec<RowJoinBindingPlan>,
}

pub(super) enum RowExistencePlan {
    Axis(AxisExistencePlan),
    Support(SupportExistencePlan),
}

pub(super) struct AxisExistencePlan {
    pub(super) source: Expr,
    pub(super) nested: Option<NestedAxisPlan>,
}

pub(super) struct SupportExistencePlan {
    pub(super) support: SupportSpec,
}

pub(super) struct SupportBindingPlan {
    pub(super) binding_index: usize,
    pub(super) binding: Ident,
}

pub(super) struct IndexJoinCardinalityPlan {
    pub(super) source: Expr,
}

pub(super) struct ZipJoinCoveragePlan {
    pub(super) source: Expr,
    pub(super) condition: Expr,
    pub(super) alias: Option<Ident>,
}

pub(super) struct RowJoinBindingPlan {
    pub(super) join_index: usize,
    pub(super) binding: Ident,
    pub(super) join_axis: Expr,
}

pub(super) struct NestedAxisPlan {
    pub(super) parent: Expr,
    pub(super) child: Member,
}
