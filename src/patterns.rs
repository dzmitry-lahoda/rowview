//! Specializaton of mattched Rust expression patters.
//! Form pure Rust based eDSL. Per se active docs for all possible features.

struct RowsArgs {
    root: Ident,
}

struct RowsModule {
    vis: Visibility,
    name: Ident,
    imports: Vec<ItemUse>,
    rowsets: Vec<RowsetSpec>,
}

struct RowsetSpec {
    rowset_name: Ident,
    axis: Expr,
    struct_name: Ident,
    fields: Vec<FieldSpec>,
}

struct FieldSpec {
    kind: FieldKind,
    mode: FieldMode,
    name: Ident,
    ty: syn::Type,
    expr: Expr,
}

struct IncrementExpr {
    expr: Expr,
}

struct NestedAxisSpec {
    parent: Expr,
    child: Member,
}