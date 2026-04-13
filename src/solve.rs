use crate::docs::{FieldKind, FieldMode, JoinKey};
use crate::generate::{join_axis_for_expr, row_join_binding_ident};
use crate::parse::parse_join_key;
use crate::schema::{
    AxisExistencePlan, BindingFilter, BindingLookup, BindingSpec, IndexJoinCardinalityPlan,
    RelationBuildPlan, RelationGenerator, RelationSchema, RowExistencePlan, RowJoinBindingPlan,
    SchemaModule, SupportBindingPlan, SupportExistencePlan, SupportSourceSpec, SupportSpec,
    ZipJoinCoveragePlan,
};
use proc_macro2::Span;
use quote::format_ident;
use syn::parse::{Parse, ParseStream};
use syn::{
    BinOp, Expr, ExprCall, ExprField, ExprIndex, ExprPath, Ident, Member, Result, Token, UnOp,
    parenthesized,
};

pub(crate) fn validate_relations(module: &SchemaModule) -> Result<Vec<RelationBuildPlan>> {
    module
        .relations
        .iter()
        .enumerate()
        .map(|(relation_index, relation)| validate_relation_build_plan(relation_index, relation))
        .collect()
}

fn validate_relation_build_plan(
    relation_index: usize,
    relation: &RelationSchema,
) -> Result<RelationBuildPlan> {
    validate_row_existence_consistency(relation)?;
    let row_existence = solve_row_existence(relation);
    validate_join_conditions(relation)?;
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
        .filter(|(join_index, join)| {
            join.skips_row_on_miss() || relation_selects_join(relation, *join_index)
        })
        .map(|(join_index, join)| {
            Ok(RowJoinBindingPlan {
                join_index,
                binding: row_join_binding_ident(join_index),
                join_axis: join_axis_for_expr(join, None)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let support_bindings = relation
        .bindings
        .iter()
        .enumerate()
        .map(|(binding_index, binding)| {
            Ok(SupportBindingPlan {
                binding_index,
                binding: support_binding_ident(&binding.alias),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    validate_binding_filter_references(&relation.bindings)?;

    Ok(RelationBuildPlan {
        relation_index,
        row_existence,
        support_bindings,
        index_join_len_asserts,
        zip_join_key_asserts,
        row_joins,
    })
}

fn validate_row_existence_consistency(relation: &RelationSchema) -> Result<()> {
    if let Some(attribute) = relation.attributes.iter().find(|attribute| {
        attribute
            .join
            .as_ref()
            .is_some_and(|join| join.skips_row_on_miss())
    }) {
        return Err(syn::Error::new(
            attribute.name.span(),
            "`inner` joins must be declared with row-level `#[joins(...)]`",
        ));
    }

    match &relation.generator {
        RelationGenerator::Axis(_) => {
            if !relation.bindings.is_empty() {
                return Err(syn::Error::new(
                    relation.relation_name.span(),
                    "`#[bind(...)]` requires `#[support(...)]` row existence",
                ));
            }
            if let Some(attribute) = relation
                .attributes
                .iter()
                .find(|attribute| matches!(attribute.kind, FieldKind::FromKey))
            {
                return Err(syn::Error::new(
                    attribute.name.span(),
                    "`#[from_key(...)]` requires `#[support(...)]` row existence",
                ));
            }
        }
        RelationGenerator::Support(support) => {
            if support.sources.is_empty() {
                return Err(syn::Error::new(
                    relation.relation_name.span(),
                    "`#[support(any(...))]` requires at least one source",
                ));
            }
            if !relation.joins.is_empty()
                || relation
                    .attributes
                    .iter()
                    .any(|attribute| attribute.join.is_some())
            {
                return Err(syn::Error::new(
                    relation.relation_name.span(),
                    "support relations use `#[bind(...)]` instead of `#[joins(...)]` or `#[join(...)]`",
                ));
            }
            if let Some(attribute) = relation.attributes.iter().find(|attribute| {
                matches!(attribute.kind, FieldKind::FromAxis | FieldKind::FromIndex)
            }) {
                return Err(syn::Error::new(
                    attribute.name.span(),
                    "support relations have no axis; use `#[from_key(...)]` or `#[select(...)]`",
                ));
            }
        }
    }

    Ok(())
}

fn validate_join_conditions(relation: &RelationSchema) -> Result<()> {
    relation
        .joins
        .iter()
        .chain(
            relation
                .attributes
                .iter()
                .filter_map(|attribute| attribute.join.as_ref()),
        )
        .filter(|join| !join.is_index())
        .filter_map(|join| join.condition.as_ref())
        .try_for_each(validate_join_key_condition)
}

fn validate_join_key_condition(condition: &Expr) -> Result<()> {
    let Expr::Binary(binary) = condition else {
        return Err(syn::Error::new_spanned(
            condition,
            "join `on = ...` must be a single id equality",
        ));
    };
    if !matches!(binary.op, BinOp::Eq(_)) {
        return Err(syn::Error::new_spanned(
            condition,
            "join `on = ...` must be a single id equality",
        ));
    }
    validate_join_key_expr(&binary.left)?;
    validate_join_key_expr(&binary.right)
}

fn validate_join_key_expr(expr: &Expr) -> Result<()> {
    match expr {
        Expr::Path(_) | Expr::Field(_) | Expr::Index(_) | Expr::Lit(_) => Ok(()),
        Expr::Paren(paren) => validate_join_key_expr(&paren.expr),
        Expr::Reference(reference) => validate_join_key_expr(&reference.expr),
        Expr::Unary(unary) if matches!(unary.op, UnOp::Deref(_)) => {
            validate_join_key_expr(&unary.expr)
        }
        Expr::Group(group) => validate_join_key_expr(&group.expr),
        _ => Err(syn::Error::new_spanned(
            expr,
            "join id equality may use only paths, fields, indexes, literals, references, and `*` dereference",
        )),
    }
}

fn solve_row_existence(relation: &RelationSchema) -> RowExistencePlan {
    match &relation.generator {
        RelationGenerator::Axis(axis) => RowExistencePlan::Axis(AxisExistencePlan {
            source: axis.clone(),
            nested: parse_nested_axis_expr(axis),
        }),
        RelationGenerator::Support(support) => RowExistencePlan::Support(SupportExistencePlan {
            support: support.clone(),
        }),
    }
}

fn relation_selects_join(relation: &RelationSchema, join_index: usize) -> bool {
    relation.attributes.iter().any(|attribute| {
        matches!(
            (&attribute.kind, &attribute.mode),
            (FieldKind::Select, FieldMode::Direct)
        ) && crate::generate::select_join_for_expr_index(&attribute.expr, &relation.joins)
            .is_some_and(|(selected_index, _)| selected_index == join_index)
    })
}

fn support_binding_ident(alias: &Ident) -> Ident {
    format_ident!("{alias}")
}

fn validate_binding_filter_references(bindings: &[BindingSpec]) -> Result<()> {
    let mut prior_aliases = Vec::new();
    for binding in bindings {
        if let Some(filter) = &binding.filter {
            validate_binding_filter_reference(filter, &prior_aliases)?;
        }
        prior_aliases.push(binding.alias.clone());
    }

    Ok(())
}

fn validate_binding_filter_reference(
    filter: &BindingFilter,
    prior_aliases: &[Ident],
) -> Result<()> {
    match filter {
        BindingFilter::Some(alias) => {
            if prior_aliases.iter().any(|prior| prior == alias) {
                Ok(())
            } else {
                Err(syn::Error::new(
                    alias.span(),
                    "`bind` dependency filters may reference only earlier bindings",
                ))
            }
        }
        BindingFilter::Any(filters) | BindingFilter::All(filters) => filters
            .iter()
            .try_for_each(|filter| validate_binding_filter_reference(filter, prior_aliases)),
        BindingFilter::Not(filter) => validate_binding_filter_reference(filter, prior_aliases),
    }
}

pub(crate) fn parse_nested_axis_expr(expr: &Expr) -> Option<crate::schema::NestedAxisPlan> {
    let Expr::Field(attribute) = expr else {
        return None;
    };
    let index = field_range_index(attribute)?;

    Some(crate::schema::NestedAxisPlan {
        parent: (*index.expr).clone(),
        child: clone_member(&attribute.member),
    })
}

pub(crate) fn parse_join_axis_expr(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Field(attribute) => parse_join_axis_field_expr(attribute),
        Expr::Binary(binary) => {
            parse_join_axis_expr(&binary.left).or_else(|| parse_join_axis_expr(&binary.right))
        }
        Expr::Paren(paren) => parse_join_axis_expr(&paren.expr),
        Expr::Reference(reference) => parse_join_axis_expr(&reference.expr),
        Expr::Unary(unary) => parse_join_axis_expr(&unary.expr),
        Expr::Group(group) => parse_join_axis_expr(&group.expr),
        _ => None,
    }
}

fn parse_join_axis_field_expr(attribute: &ExprField) -> Option<Expr> {
    field_range_index(attribute)
        .map(|index| (*index.expr).clone())
        .or_else(|| parse_join_axis_expr(&attribute.base))
}

fn field_range_index(attribute: &ExprField) -> Option<&ExprIndex> {
    let Expr::Index(index) = &*attribute.base else {
        return None;
    };
    matches!(&*index.index, Expr::Range(_)).then_some(index)
}

fn clone_member(member: &Member) -> Member {
    match member {
        Member::Named(ident) => Member::Named(Ident::new(&ident.to_string(), ident.span())),
        Member::Unnamed(index) => Member::Unnamed(syn::Index {
            index: index.index,
            span: Span::call_site(),
        }),
    }
}

impl Parse for SupportSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key: Ident = input.parse()?;
        if key != "any" {
            return Err(syn::Error::new(key.span(), "expected `any(...)`"));
        }

        let content;
        parenthesized!(content in input);
        let mut sources = Vec::new();
        while !content.is_empty() {
            sources.push(content.parse()?);
            if content.is_empty() {
                break;
            }
            content.parse::<Token![,]>()?;
        }

        Ok(Self { sources })
    }
}

impl Parse for SupportSourceSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key = input.parse()?;
        let source = parse_join_axis_expr(&key).ok_or_else(|| {
            syn::Error::new_spanned(
                &key,
                "support source must reference a collection slice like `root.values[..].id`",
            )
        })?;

        Ok(Self { source, key })
    }
}

impl Parse for BindingSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut source = None;
        let mut alias = None;
        let mut key_expr = None;
        let mut filter = None;

        while !input.is_empty() {
            let key = parse_join_key(input)?;
            input.parse::<Token![=]>()?;
            match key {
                JoinKey::Left | JoinKey::From => source = Some(input.parse()?),
                JoinKey::As | JoinKey::Alias => alias = Some(input.parse()?),
                JoinKey::By => key_expr = Some(input.parse()?),
                JoinKey::Option | JoinKey::On => {
                    let expr: Expr = input.parse()?;
                    filter = Some(parse_binding_filter_expr(&expr)?);
                }
                other => {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!("unsupported bind key `{}`", other.as_ref()),
                    ));
                }
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        let lookup = match key_expr {
            Some(expr) => BindingLookup::Key { expr },
            None => {
                return Err(input.error("bind requires `by = ...`; `on = ...` may only add an any/all/not dependency filter"));
            }
        };

        Ok(Self {
            source: source
                .ok_or_else(|| input.error("bind requires `left = ...` or `from = ...`"))?,
            alias: alias.ok_or_else(|| input.error("bind requires `as = alias`"))?,
            lookup,
            filter,
        })
    }
}

fn parse_binding_filter_expr(expr: &Expr) -> Result<BindingFilter> {
    match expr {
        Expr::Path(path) => parse_binding_filter_path(path),
        Expr::Call(call) => parse_binding_filter_call(call),
        Expr::Paren(paren) => parse_binding_filter_expr(&paren.expr),
        Expr::Group(group) => parse_binding_filter_expr(&group.expr),
        _ => Err(syn::Error::new_spanned(
            expr,
            "`bind` filters support only binding aliases and `any(...)`, `all(...)`, `not(...)`",
        )),
    }
}

fn parse_binding_filter_path(path: &ExprPath) -> Result<BindingFilter> {
    if path.qself.is_some() || path.path.segments.len() != 1 {
        return Err(syn::Error::new_spanned(
            path,
            "`bind` filters support only bare binding aliases",
        ));
    }

    Ok(BindingFilter::Some(path.path.segments[0].ident.clone()))
}

fn parse_binding_filter_call(call: &ExprCall) -> Result<BindingFilter> {
    let Expr::Path(path) = &*call.func else {
        return Err(syn::Error::new_spanned(
            call,
            "`bind` filter calls must be `any(...)`, `all(...)`, or `not(...)`",
        ));
    };
    if path.qself.is_some() || path.path.segments.len() != 1 {
        return Err(syn::Error::new_spanned(
            call,
            "`bind` filter calls must be `any(...)`, `all(...)`, or `not(...)`",
        ));
    }

    let function = path.path.segments[0].ident.to_string();
    match function.as_str() {
        "any" => {
            let filters = parse_binding_filter_args(call)?;
            if filters.is_empty() {
                return Err(syn::Error::new_spanned(
                    call,
                    "`any(...)` requires at least one term",
                ));
            }
            Ok(BindingFilter::Any(filters))
        }
        "all" => {
            let filters = parse_binding_filter_args(call)?;
            if filters.is_empty() {
                return Err(syn::Error::new_spanned(
                    call,
                    "`all(...)` requires at least one term",
                ));
            }
            Ok(BindingFilter::All(filters))
        }
        "not" => {
            if call.args.len() != 1 {
                return Err(syn::Error::new_spanned(
                    call,
                    "`not(...)` requires exactly one term",
                ));
            }
            Ok(BindingFilter::Not(Box::new(parse_binding_filter_expr(
                &call.args[0],
            )?)))
        }
        _ => Err(syn::Error::new_spanned(
            &call.func,
            "`bind` filter calls must be `any(...)`, `all(...)`, or `not(...)`",
        )),
    }
}

fn parse_binding_filter_args(call: &ExprCall) -> Result<Vec<BindingFilter>> {
    call.args.iter().map(parse_binding_filter_expr).collect()
}
