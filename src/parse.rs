use crate::docs::{AXIS_ATTR, FieldKind, FieldMode, JoinKey, NAME_ATTR, ROOT_ATTR, ROWSET_ATTR};
use crate::generate::{
    join_axis_for_expr, parse_nested_axis_expr, row_join_binding_ident, select_join_for_expr_index,
};
use crate::schema::{
    AttributeSpec, DatabaseBuildPlan, IncrementExpr, IndexJoinCardinalityPlan, JoinLookup,
    JoinRequirement, JoinSpec, RelationBuildPlan, RelationSchema, RowJoinBindingPlan, RowViewArgs,
    SchemaModule, ZipJoinCoveragePlan,
};
use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Item, ItemStruct, Result, Token, braced};

impl Parse for RowViewArgs {
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

impl Parse for SchemaModule {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let vis = input.parse()?;
        input.parse::<Token![mod]>()?;
        let name: Ident = input.parse()?;
        let content;
        braced!(content in input);

        let mut imports = Vec::new();
        let mut relations = Vec::new();
        while !content.is_empty() {
            match content.parse::<Item>()? {
                Item::Use(item_use) => imports.push(item_use),
                Item::Struct(item_struct) => {
                    relations.push(RelationSchema::from_item_struct(item_struct)?)
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
            relations,
        })
    }
}

impl RelationSchema {
    fn from_item_struct(item_struct: ItemStruct) -> Result<Self> {
        let rust_attributes = item_struct.attrs;
        let struct_name = item_struct.ident;
        let mut attributes = Vec::new();
        for attribute in item_struct.fields {
            let name = attribute
                .ident
                .ok_or_else(|| syn::Error::new(struct_name.span(), "expected named field"))?;
            let ty = attribute.ty;
            let rust_attributes = attribute.attrs;
            let name_span = name.span();

            let mut kind = None;
            let mut mode = FieldMode::Direct;
            let mut expr = None;
            let mut agg_convert_into = false;
            let mut join = None;
            for attr in rust_attributes {
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
                    let spec = attr.parse_args::<JoinSpec>()?;
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

            attributes.push(AttributeSpec {
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

        let mut relation_name = None;
        let mut generator = None;
        let mut joins = Vec::new();
        let mut row_attrs = Vec::new();
        for attr in rust_attributes {
            if attr.path().is_ident(ROWSET_ATTR) {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident(NAME_ATTR) {
                        relation_name = Some(meta.value()?.parse()?);
                        return Ok(());
                    }
                    if meta.path.is_ident(AXIS_ATTR) {
                        generator = Some(meta.value()?.parse()?);
                        return Ok(());
                    }
                    Err(meta.error("unsupported relation attribute"))
                })?;
            } else if attr.path().is_ident("joins") {
                joins.push(attr.parse_args()?);
            } else {
                row_attrs.push(attr);
            }
        }

        Ok(Self {
            rust_attributes: row_attrs,
            joins,
            relation_name: relation_name.ok_or_else(|| {
                syn::Error::new(struct_name.span(), format!("missing `{NAME_ATTR}`"))
            })?,
            generator: generator.ok_or_else(|| {
                syn::Error::new(struct_name.span(), format!("missing `{AXIS_ATTR}`"))
            })?,
            struct_name,
            attributes,
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

impl Parse for JoinSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut source = None;
        let mut alias = None;
        let mut condition = None;
        let mut lookup = JoinLookup::Predicate;
        let mut requirement = JoinRequirement::Optional;
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
                    requirement = JoinRequirement::Required;
                    source = Some(input.parse()?);
                }
                JoinKey::Zip => {
                    lookup = JoinLookup::Zip;
                    requirement = JoinRequirement::Required;
                    source = Some(input.parse()?);
                }
                JoinKey::Index => {
                    lookup = JoinLookup::Index;
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
            condition: if matches!(lookup, JoinLookup::Index) {
                condition
            } else {
                Some(condition.ok_or_else(|| input.error("missing join condition (`on = ...`)"))?)
            },
            lookup,
            requirement,
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

pub(crate) fn validate_rows(args: RowViewArgs, module: SchemaModule) -> Result<DatabaseBuildPlan> {
    let relations = module
        .relations
        .iter()
        .enumerate()
        .map(|(relation_index, relation)| validate_relation_build_plan(relation_index, relation))
        .collect::<Result<Vec<_>>>()?;

    Ok(DatabaseBuildPlan {
        args,
        module,
        relations,
    })
}

fn validate_relation_build_plan(
    relation_index: usize,
    relation: &RelationSchema,
) -> Result<RelationBuildPlan> {
    let nested_generator = parse_nested_axis_expr(&relation.generator);
    let joins = || {
        relation.joins.iter().chain(
            relation
                .attributes
                .iter()
                .filter_map(|attribute| attribute.join.as_ref()),
        )
    };
    let index_join_len_asserts = joins()
        .filter(|join| join.is_index())
        .map(|join| {
            Ok(IndexJoinCardinalityPlan {
                source: join.source.clone().ok_or_else(|| {
                    syn::Error::new(Span::call_site(), "index join requires source")
                })?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let zip_join_key_asserts =
        joins()
            .filter(|join| join.is_zip())
            .map(|join| {
                let source = join.source.clone().ok_or_else(|| {
                    syn::Error::new(Span::call_site(), "zip join requires source")
                })?;
                let condition = join.condition.clone().ok_or_else(|| {
                    syn::Error::new_spanned(&source, "zip join requires `on = ...`")
                })?;
                Ok(ZipJoinCoveragePlan {
                    source,
                    condition,
                    alias: join.alias.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
    let row_joins = relation
        .joins
        .iter()
        .enumerate()
        .filter(|(join_index, _)| relation_selects_join(relation, *join_index))
        .map(|(join_index, join)| {
            Ok(RowJoinBindingPlan {
                join_index,
                binding: row_join_binding_ident(join_index),
                join_axis: join_axis_for_expr(join, None)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(RelationBuildPlan {
        relation_index,
        nested_generator,
        index_join_len_asserts,
        zip_join_key_asserts,
        row_joins,
    })
}

fn relation_selects_join(relation: &RelationSchema, join_index: usize) -> bool {
    relation.attributes.iter().any(|attribute| {
        matches!(
            (&attribute.kind, &attribute.mode),
            (FieldKind::Select, FieldMode::Direct)
        ) && select_join_for_expr_index(&attribute.expr, &relation.joins)
            .is_some_and(|(selected_index, _)| selected_index == join_index)
    })
}
