use crate::docs::{AXIS_ATTR, FieldKind, FieldMode, JoinKey, NAME_ATTR, ROOT_ATTR, ROWSET_ATTR};
use crate::generate::{
    join_axis_for_expr, parse_nested_axis_expr, row_join_binding_ident, select_join_for_expr_index,
};
use crate::schema::{
    FieldSpec, IncrementExpr, IndexJoinLenAssertPlan, JoinOptionSpec, RowJoinPlan, RowsArgs,
    RowsBuildPlan, RowsModule, RowsetBuildPlan, RowsetSpec, ZipJoinKeyAssertPlan,
};
use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Item, ItemStruct, Result, Token, braced};

impl Parse for RowsArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key: Ident = input.parse()?;
        if key != ROOT_ATTR {
            return Err(syn::Error::new(
                key.span(),
                format!("expected `{ROOT_ATTR} = Ident`"),
            ));
        }
        input.parse::<Token![=]>()?;
        Ok(Self {
            root: input.parse()?,
        })
    }
}

impl Parse for RowsModule {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let vis = input.parse()?;
        input.parse::<Token![mod]>()?;
        let name: Ident = input.parse()?;
        let content;
        braced!(content in input);

        let mut imports = Vec::new();
        let mut rowsets = Vec::new();
        while !content.is_empty() {
            match content.parse::<Item>()? {
                Item::Use(item_use) => imports.push(item_use),
                Item::Struct(item_struct) => {
                    rowsets.push(RowsetSpec::from_item_struct(item_struct)?)
                }
                item => {
                    return Err(syn::Error::new_spanned(
                        item,
                        "expected `use` or `struct` item",
                    ));
                }
            }
        }

        Ok(Self {
            vis,
            name,
            imports,
            rowsets,
        })
    }
}

impl RowsetSpec {
    fn from_item_struct(item_struct: ItemStruct) -> Result<Self> {
        let attrs = item_struct.attrs;
        let struct_name = item_struct.ident;
        let mut fields = Vec::new();
        for field in item_struct.fields {
            let name = field
                .ident
                .ok_or_else(|| syn::Error::new(struct_name.span(), "expected named field"))?;
            let ty = field.ty;
            let attrs = field.attrs;
            let name_span = name.span();

            let mut kind = None;
            let mut mode = FieldMode::Direct;
            let mut expr = None;
            let mut agg_convert_into = false;
            let mut join = None;
            for attr in attrs {
                if attr.path().is_ident(FieldKind::Copy.as_ref()) {
                    kind = Some(FieldKind::Copy);
                    if let Ok(named) = attr.parse_args::<IncrementExpr>() {
                        mode = FieldMode::Increment;
                        expr = Some(named.expr);
                    } else {
                        expr = Some(attr.parse_args()?);
                    }
                }
                if attr.path().is_ident(FieldKind::Agg.as_ref()) {
                    kind = Some(FieldKind::Agg);
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("sum") {
                            expr = Some(meta.value()?.parse()?);
                            return Ok(());
                        }
                        if meta.path.is_ident("convert") {
                            let convert: Ident = meta.value()?.parse()?;
                            if convert == "into" {
                                agg_convert_into = true;
                                return Ok(());
                            }
                            return Err(meta.error("expected `convert = into`"));
                        }
                        if meta.path.is_ident("into") {
                            let _: syn::Type = meta.value()?.parse()?;
                            return Err(meta.error(
                                "use `convert = into`; target type is inferred from the field type",
                            ));
                        }
                        Err(meta.error("unsupported agg attribute"))
                    })?;
                }
                if attr.path().is_ident(FieldKind::FromAxis.as_ref()) {
                    kind = Some(FieldKind::FromAxis);
                    expr = Some(attr.parse_args()?);
                }
                if attr.path().is_ident(FieldKind::FromIndex.as_ref()) {
                    kind = Some(FieldKind::FromIndex);
                    expr = Some(attr.parse_args()?);
                }
                if attr.path().is_ident(FieldKind::Join.as_ref()) {
                    kind = Some(FieldKind::Join);
                    let spec = attr.parse_args::<JoinOptionSpec>()?;
                    expr = Some(spec.value.clone().ok_or_else(|| {
                        syn::Error::new(name_span, "missing join projection (`select = ...`)")
                    })?);
                    join = Some(spec);
                }
                if attr.path().is_ident(FieldKind::Select.as_ref()) {
                    kind = Some(FieldKind::Select);
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("select") {
                            expr = Some(meta.value()?.parse()?);
                            return Ok(());
                        }
                        Err(meta.error("unsupported select attribute"))
                    })?;
                }
            }

            fields.push(FieldSpec {
                kind: kind
                    .ok_or_else(|| syn::Error::new(name.span(), "missing field attribute"))?,
                mode,
                name,
                ty,
                expr: expr
                    .ok_or_else(|| syn::Error::new(name_span, "missing source expression"))?,
                agg_convert_into,
                join,
            });
        }

        let mut rowset_name = None;
        let mut axis = None;
        let mut joins = Vec::new();
        let mut row_attrs = Vec::new();
        for attr in attrs {
            if attr.path().is_ident(ROWSET_ATTR) {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident(NAME_ATTR) {
                        rowset_name = Some(meta.value()?.parse()?);
                        return Ok(());
                    }
                    if meta.path.is_ident(AXIS_ATTR) {
                        axis = Some(meta.value()?.parse()?);
                        return Ok(());
                    }
                    Err(meta.error("unsupported rowset attribute"))
                })?;
            } else if attr.path().is_ident("joins") {
                joins.push(attr.parse_args()?);
            } else {
                row_attrs.push(attr);
            }
        }

        Ok(Self {
            attrs: row_attrs,
            joins,
            rowset_name: rowset_name.ok_or_else(|| {
                syn::Error::new(struct_name.span(), format!("missing `{NAME_ATTR}`"))
            })?,
            axis: axis.ok_or_else(|| {
                syn::Error::new(struct_name.span(), format!("missing `{AXIS_ATTR}`"))
            })?,
            struct_name,
            fields,
        })
    }
}

impl Parse for IncrementExpr {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key: Ident = input.parse()?;
        if key != {
            let this = &FieldMode::Increment;
            this.as_ref()
        } {
            return Err(syn::Error::new(
                key.span(),
                format!("expected `{}` = expr", FieldMode::Increment.as_ref()),
            ));
        }
        input.parse::<Token![=]>()?;
        Ok(Self {
            expr: input.parse()?,
        })
    }
}

impl Parse for JoinOptionSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut source = None;
        let mut alias = None;
        let mut condition = None;
        let mut by_index = false;
        let mut required = false;
        let mut zipped = false;
        let mut value = None;

        let starts_with_key = {
            let fork = input.fork();
            parse_join_key(&fork).is_ok() && fork.peek(Token![=])
        };

        if !starts_with_key {
            source = Some(input.parse()?);
            input.parse::<Token![,]>()?;
        }

        while !input.is_empty() {
            let key = parse_join_key(input)?;
            input.parse::<Token![=]>()?;
            match key {
                JoinKey::Left | JoinKey::From => source = Some(input.parse()?),
                JoinKey::Must => {
                    required = true;
                    source = Some(input.parse()?);
                }
                JoinKey::Zip => {
                    required = true;
                    zipped = true;
                    source = Some(input.parse()?);
                }
                JoinKey::Index => {
                    by_index = true;
                    source = Some(input.parse()?);
                }
                JoinKey::As | JoinKey::Alias => alias = Some(input.parse()?),
                JoinKey::Option | JoinKey::On => condition = Some(input.parse()?),
                JoinKey::Value | JoinKey::Select => value = Some(input.parse()?),
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(Self {
            source,
            alias,
            condition: if by_index {
                condition
            } else {
                Some(condition.ok_or_else(|| input.error("missing join condition (`on = ...`)"))?)
            },
            by_index,
            required,
            zipped,
            value,
        })
    }
}

fn parse_join_key(input: ParseStream<'_>) -> Result<JoinKey> {
    if input.peek(Token![as]) {
        input.parse::<Token![as]>()?;
        Ok(JoinKey::As)
    } else {
        input.parse::<Ident>()?.try_into()
    }
}

pub(crate) fn validate_rows(args: RowsArgs, module: RowsModule) -> Result<RowsBuildPlan> {
    let rowsets = module
        .rowsets
        .iter()
        .enumerate()
        .map(|(rowset_index, rowset)| validate_rowset_build_plan(rowset_index, rowset))
        .collect::<Result<Vec<_>>>()?;

    Ok(RowsBuildPlan {
        args,
        module,
        rowsets,
    })
}

fn validate_rowset_build_plan(rowset_index: usize, rowset: &RowsetSpec) -> Result<RowsetBuildPlan> {
    let nested_axis = parse_nested_axis_expr(&rowset.axis);
    let joins = || {
        rowset
            .joins
            .iter()
            .chain(rowset.fields.iter().filter_map(|field| field.join.as_ref()))
    };
    let index_join_len_asserts = joins()
        .filter(|join| join.by_index)
        .map(|join| {
            Ok(IndexJoinLenAssertPlan {
                source: join.source.clone().ok_or_else(|| {
                    syn::Error::new(Span::call_site(), "index join requires source")
                })?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let zip_join_key_asserts =
        joins()
            .filter(|join| join.zipped)
            .map(|join| {
                let source = join.source.clone().ok_or_else(|| {
                    syn::Error::new(Span::call_site(), "zip join requires source")
                })?;
                let condition = join.condition.clone().ok_or_else(|| {
                    syn::Error::new_spanned(&source, "zip join requires `on = ...`")
                })?;
                Ok(ZipJoinKeyAssertPlan {
                    source,
                    condition,
                    alias: join.alias.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
    let row_joins = rowset
        .joins
        .iter()
        .enumerate()
        .filter(|(join_index, _)| rowset_selects_join(rowset, *join_index))
        .map(|(join_index, join)| {
            Ok(RowJoinPlan {
                join_index,
                binding: row_join_binding_ident(join_index),
                join_axis: join_axis_for_expr(join, None)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(RowsetBuildPlan {
        rowset_index,
        nested_axis,
        index_join_len_asserts,
        zip_join_key_asserts,
        row_joins,
    })
}

fn rowset_selects_join(rowset: &RowsetSpec, join_index: usize) -> bool {
    rowset.fields.iter().any(|field| {
        matches!(
            (&field.kind, &field.mode),
            (FieldKind::Select, FieldMode::Direct)
        ) && select_join_for_expr_index(&field.expr, &rowset.joins)
            .is_some_and(|(selected_index, _)| selected_index == join_index)
    })
}
