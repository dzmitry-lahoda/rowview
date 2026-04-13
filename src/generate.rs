use crate::docs::{
    AXIS_ATTR, FieldKind, FieldMode, INCREMENT_BINDING_PREFIX, ROOT_ATTR, ROWS_SUFFIX,
};
use crate::schema::{DatabaseBuildPlan, JoinSpec, NestedAxisPlan, RowJoinBindingPlan};
use heck::ToUpperCamelCase;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprBinary, ExprCall, ExprClosure, ExprField, ExprGroup, ExprIndex, ExprMethodCall,
    ExprParen, ExprPath, ExprReference, ExprUnary, Ident, Member, Result,
};

pub(crate) fn expand_rows(plan: DatabaseBuildPlan) -> Result<proc_macro2::TokenStream> {
    let root = plan.args.root;
    let module = plan.module;
    let module_vis = module.vis;
    let module_name = module.name;
    let module_imports = module.imports;
    let rows_type = format_ident!(
        "{}{ROWS_SUFFIX}",
        module_name.to_string().to_upper_camel_case()
    );

    let relation_structs = module.relations.iter().map(|relation| {
        let rust_attributes = &relation.rust_attributes;
        let struct_name = &relation.struct_name;
        let field_defs = relation.attributes.iter().map(|attribute| {
            let name = &attribute.name;
            let ty = &attribute.ty;
            quote! { pub #name: #ty }
        });
        quote! {
            #( #rust_attributes )*
            #[derive(Clone, Debug, PartialEq)]
            pub struct #struct_name {
                #( #field_defs, )*
            }
        }
    });

    let relation_fields = module.relations.iter().map(|relation| {
        let relation_name = &relation.relation_name;
        let struct_name = &relation.struct_name;
        quote! { pub #relation_name: ::std::vec::Vec<#struct_name> }
    });

    let builders = plan.relations.iter().map(|relation_plan| -> Result<_> {
        let relation = &module.relations[relation_plan.relation_index];
        let relation_name = &relation.relation_name;
        let nested_generator = relation_plan.nested_generator.as_ref();
        let generator_iter = rewrite_axis_iter_expr(&relation.generator, nested_generator);
        let relation_joins = &relation.joins;
        let index_join_len_asserts = relation_plan
            .index_join_len_asserts
            .iter()
            .map(|index_assert| {
                let generator = rewrite_source_expr(&relation.generator, ROOT_ATTR, quote! { self });
                let join_source = rewrite_source_expr(&index_assert.source, ROOT_ATTR, quote! { self });
                quote! {
                    assert_eq!(
                        (#generator).len(),
                        (#join_source).len(),
                        "rowview index join requires axis and joined collection lengths to match"
                    );
                }
            })
            .collect::<Vec<_>>();
        let zip_join_key_asserts = relation_plan
            .zip_join_key_asserts
            .iter()
            .map(|zip_assert| {
                let join_iter = rewrite_source_expr(&zip_assert.source, ROOT_ATTR, quote! { self });
                let generator_iter = rewrite_axis_iter_expr(&relation.generator, nested_generator);
                let condition = rewrite_join_context_expr(
                    &zip_assert.condition,
                    nested_generator,
                    &zip_assert.source,
                    zip_assert.alias.as_ref(),
                );
                quote! {
                    assert!(
                        (#join_iter)
                            .iter()
                            .all(|join_item| (#generator_iter).any(|(generator_parent, generator_item)| #condition)),
                        "rowview zip join found joined item with no matching axis item"
                    );
                }
            })
            .collect::<Vec<_>>();
        let struct_name = &relation.struct_name;
        let qualified_struct_name = quote! { #module_name::#struct_name };
        let increment_bindings = relation.attributes.iter().filter_map(|attribute| {
            if !matches!(attribute.mode, FieldMode::Increment) {
                return None;
            }
            let binding = format_ident!("{INCREMENT_BINDING_PREFIX}{}", attribute.name);
            let value = rewrite_row_expr(&attribute.expr, nested_generator);
            Some(quote! {
                let mut #binding = #value;
            })
        });
        let field_inits = relation.attributes.iter().map(|attribute| -> Result<_> {
            let name = &attribute.name;
            let value = match (&attribute.kind, &attribute.mode) {
                (FieldKind::Copy, FieldMode::Direct) | (FieldKind::FromAxis, FieldMode::Direct) => {
                    rewrite_row_expr(&attribute.expr, nested_generator)
                }
                (FieldKind::Agg, FieldMode::Direct) => {
                    let ty = &attribute.ty;
                    if let Some(join) = select_join_for_expr(&attribute.expr, relation_joins) {
                        rewrite_join_agg_sum_expr(join, &attribute.expr, nested_generator, ty)?
                    } else {
                        let values = rewrite_row_expr(&attribute.expr, nested_generator);
                        rewrite_agg_sum_iter_expr(values, ty, attribute.agg_convert_into)
                    }
                }
                (FieldKind::FromIndex, FieldMode::Direct) => {
                    quote! {
                        generator_index
                            .try_into()
                            .expect("rowview axis index exceeds target field type capacity")
                    }
                }
                (FieldKind::Join, FieldMode::Direct) => {
                    rewrite_join_expr(attribute.join.as_ref().expect("join field has spec"), nested_generator)?
                }
                (FieldKind::Select, FieldMode::Direct) => {
                    let (join_index, _) = select_join_for_expr_index(&attribute.expr, relation_joins)
                        .ok_or_else(|| syn::Error::new_spanned(&attribute.expr, "select field requires a matching row-level `#[joins(...)]`"))?;
                    let row_join_plan = relation_plan
                        .row_joins
                        .iter()
                        .find(|row_join| row_join.join_index == join_index)
                        .ok_or_else(|| syn::Error::new_spanned(&attribute.expr, "select field requires a planned row-level join"))?;
                    rewrite_join_select_expr(&relation_joins[join_index], &attribute.expr, nested_generator, Some(&row_join_plan.binding))?
                }
                (FieldKind::Copy, FieldMode::Increment) => {
                    let binding = format_ident!("{INCREMENT_BINDING_PREFIX}{}", attribute.name);
                    quote! {{
                        let value = #binding;
                        #binding += 1;
                        value
                    }}
                }
                (FieldKind::Agg | FieldKind::FromAxis | FieldKind::FromIndex | FieldKind::Join | FieldKind::Select, FieldMode::Increment) => unreachable!(),
            };
            Ok(quote! { #name: #value })
        }).collect::<Result<Vec<_>>>()?;
        let row_join_bindings = relation_plan.row_joins
            .iter()
            .map(|row_join| {
                rewrite_row_join_plan(row_join, &relation_joins[row_join.join_index], nested_generator)
            })
            .collect::<Result<Vec<_>>>()?;

        let relation_values = if matches!(&relation.generator, Expr::Tuple(tuple) if tuple.elems.is_empty()) {
            quote! {
                {
                    #( #increment_bindings )*
                    ::std::iter::once(#qualified_struct_name {
                        #( #field_inits, )*
                    }).collect()
                }
            }
        } else {
            quote! {
                {
                    #( #index_join_len_asserts )*
                    #( #zip_join_key_asserts )*
                    #( #increment_bindings )*
                    #generator_iter.enumerate().map(|(generator_index, generator_item)| {
                        let (generator_parent, generator_item) = generator_item;
                        #( #row_join_bindings )*
                        #qualified_struct_name {
                            #( #field_inits, )*
                        }
                    }).collect()
                }
            }
        };

        Ok(quote! {
            #relation_name: #relation_values
        })
    }).collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #module_vis mod #module_name {
            #( #module_imports )*
            #( #relation_structs )*

            #[derive(Clone, Debug, PartialEq)]
            pub struct #rows_type {
                #( #relation_fields, )*
            }
        }

        #[forbid(clippy::clone_on_copy, clippy::redundant_clone, clippy::unwrap_used)]
        impl #root {
            pub fn to_rows(&self) -> #module_name::#rows_type {
                #module_name::#rows_type {
                    #( #builders, )*
                }
            }
        }
    })
}

fn rewrite_source_expr(
    expr: &Expr,
    base_name: &str,
    replacement: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    rewrite_context_expr(expr, &[(base_name, replacement)])
}

fn rewrite_axis_iter_expr(
    expr: &Expr,
    nested_generator: Option<&NestedAxisPlan>,
) -> proc_macro2::TokenStream {
    if let Some(iter) = rewrite_nested_axis_iter_expr(nested_generator) {
        iter
    } else {
        let generator = rewrite_source_expr(expr, ROOT_ATTR, quote! { self });
        quote! { (#generator).iter().map(|generator_item| { ((), generator_item) }) }
    }
}

pub(crate) fn parse_nested_axis_expr(expr: &Expr) -> Option<NestedAxisPlan> {
    let Expr::Field(attribute) = expr else {
        return None;
    };
    let index = field_range_index(attribute)?;

    Some(NestedAxisPlan {
        parent: (*index.expr).clone(),
        child: clone_member(&attribute.member),
    })
}

fn rewrite_nested_axis_iter_expr(
    nested_generator: Option<&NestedAxisPlan>,
) -> Option<proc_macro2::TokenStream> {
    let nested_generator = nested_generator?;
    let base = rewrite_source_expr(&nested_generator.parent, ROOT_ATTR, quote! { self });
    let member = clone_member(&nested_generator.child);
    Some(quote! {
        (#base).iter().flat_map(|generator_parent| {
            generator_parent.#member.iter().map(move |generator_item| (generator_parent, generator_item))
        })
    })
}

fn rewrite_row_expr(
    expr: &Expr,
    nested_generator: Option<&NestedAxisPlan>,
) -> proc_macro2::TokenStream {
    if let Some(nested_generator) = nested_generator
        && let Some(parent_expr) = rewrite_parent_expr(expr, nested_generator)
    {
        return parent_expr;
    }

    rewrite_context_expr(
        expr,
        &[
            (ROOT_ATTR, quote! { self }),
            (AXIS_ATTR, quote! { generator_item }),
        ],
    )
}

fn rewrite_parent_expr(
    expr: &Expr,
    nested_generator: &NestedAxisPlan,
) -> Option<proc_macro2::TokenStream> {
    let Expr::Field(attribute) = expr else {
        return None;
    };
    let index = field_range_index(attribute)?;
    let parent = rewrite_source_expr(&nested_generator.parent, ROOT_ATTR, quote! { self });
    let requested_parent = rewrite_source_expr(&index.expr, ROOT_ATTR, quote! { self });
    if parent.to_string() != requested_parent.to_string() {
        return None;
    }
    let member = clone_member(&attribute.member);
    Some(quote! { generator_parent.#member })
}

fn field_range_index(attribute: &ExprField) -> Option<&ExprIndex> {
    let Expr::Index(index) = &*attribute.base else {
        return None;
    };
    matches!(&*index.index, Expr::Range(_)).then_some(index)
}

fn rewrite_join_expr(
    join: &JoinSpec,
    nested_generator: Option<&NestedAxisPlan>,
) -> Result<proc_macro2::TokenStream> {
    let value = join.value.as_ref().ok_or_else(|| {
        let span_expr = join.condition.as_ref().or(join.source.as_ref());
        span_expr.map_or_else(
            || syn::Error::new(Span::call_site(), "join field requires `select = ...`"),
            |expr| syn::Error::new_spanned(expr, "join field requires `select = ...`"),
        )
    })?;
    rewrite_join_select_expr(join, value, nested_generator, None)
}

pub(crate) fn row_join_binding_ident(join_index: usize) -> Ident {
    format_ident!("__rowview_join_{join_index}")
}

fn rewrite_agg_sum_iter_expr(
    values: proc_macro2::TokenStream,
    ty: &syn::Type,
    convert_into: bool,
) -> proc_macro2::TokenStream {
    if convert_into {
        return quote! {
            (#values)
                .iter()
                .map(|value| ::core::convert::Into::<#ty>::into(*value))
                .sum::<#ty>()
        };
    }

    quote! {
        (#values)
            .iter()
            .map(|value| ::core::iter::once(*value).sum::<#ty>())
            .sum::<#ty>()
    }
}

fn rewrite_join_agg_sum_expr(
    join: &JoinSpec,
    value_expr: &Expr,
    nested_generator: Option<&NestedAxisPlan>,
    ty: &syn::Type,
) -> Result<proc_macro2::TokenStream> {
    let join_axis = join
        .source
        .clone()
        .or_else(|| {
            join.condition
                .as_ref()
                .and_then(parse_join_axis_expr)
        })
        .or_else(|| parse_join_axis_expr(value_expr))
        .ok_or_else(|| syn::Error::new_spanned(
            value_expr,
            "agg expression must provide a join source like `#[joins(left = root.values[..], as = vals, on = ...)]`",
        ))?;
    let condition_expr = join
        .condition
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(value_expr, "join agg requires `on = ...`"))?;
    let join_iter = rewrite_source_expr(&join_axis, ROOT_ATTR, quote! { self });
    let condition = rewrite_join_context_expr(
        condition_expr,
        nested_generator,
        &join_axis,
        join.alias.as_ref(),
    );
    let value = rewrite_join_context_expr(
        value_expr,
        nested_generator,
        &join_axis,
        join.alias.as_ref(),
    );

    Ok(quote! {
        (#join_iter)
            .iter()
            .filter(|join_item| #condition)
            .map(|join_item| ::core::convert::Into::<#ty>::into(#value))
            .sum::<#ty>()
    })
}

fn rewrite_join_lookup_expr(
    join: &JoinSpec,
    join_axis: &Expr,
    nested_generator: Option<&NestedAxisPlan>,
) -> Result<proc_macro2::TokenStream> {
    let join_source = rewrite_source_expr(&join_axis, ROOT_ATTR, quote! { self });
    if join.is_index() {
        return Ok(quote! {
            (#join_source).get(generator_index)
        });
    }

    let condition_expr = join
        .condition
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(&join_axis, "join lookup requires `on = ...`"))?;
    let condition = rewrite_join_context_expr(
        condition_expr,
        nested_generator,
        &join_axis,
        join.alias.as_ref(),
    );

    Ok(quote! {
        (#join_source).iter().filter(|join_item| #condition).last()
    })
}

fn rewrite_row_join_plan(
    plan: &RowJoinBindingPlan,
    join: &JoinSpec,
    nested_generator: Option<&NestedAxisPlan>,
) -> Result<proc_macro2::TokenStream> {
    let binding = &plan.binding;
    let value = rewrite_join_lookup_expr(join, &plan.join_axis, nested_generator)?;
    Ok(quote! {
        let #binding = #value;
    })
}

fn rewrite_join_select_expr(
    join: &JoinSpec,
    value_expr: &Expr,
    nested_generator: Option<&NestedAxisPlan>,
    binding: Option<&Ident>,
) -> Result<proc_macro2::TokenStream> {
    let join_axis = join_axis_for_expr(join, Some(value_expr))?;
    let value = if join.is_index() {
        rewrite_index_join_context_expr(value_expr, nested_generator, join.alias.as_ref())
    } else {
        rewrite_join_context_expr(
            value_expr,
            nested_generator,
            &join_axis,
            join.alias.as_ref(),
        )
    };
    let lookup = if let Some(binding) = binding {
        quote! { #binding }
    } else {
        rewrite_join_lookup_expr(join, &join_axis, nested_generator)?
    };

    Ok(rewrite_join_select_value(lookup, value, join.is_required()))
}

fn rewrite_join_select_value(
    lookup: proc_macro2::TokenStream,
    value: proc_macro2::TokenStream,
    required: bool,
) -> proc_macro2::TokenStream {
    let selected = quote! {
        (#lookup).and_then(|join_item| ::core::option::Option::Some(#value))
    };
    if required {
        quote! { #selected.expect("rowview must join found no matching item") }
    } else {
        selected
    }
}

pub(crate) fn join_axis_for_expr(join: &JoinSpec, value_expr: Option<&Expr>) -> Result<Expr> {
    join.source
        .clone()
        .or_else(|| join.condition.as_ref().and_then(parse_join_axis_expr))
        .or_else(|| value_expr.and_then(parse_join_axis_expr))
        .ok_or_else(|| {
            let message = "join expression must provide a source like `#[join(root.values[..], on = ..., select = ...)]` or reference a collection slice like `root.values[..].field`";
            value_expr
                .or(join.condition.as_ref())
                .or(join.source.as_ref())
                .map_or_else(
                    || syn::Error::new(Span::call_site(), message),
                    |expr| syn::Error::new_spanned(expr, message),
                )
        })
}

fn rewrite_index_join_context_expr(
    expr: &Expr,
    nested_generator: Option<&NestedAxisPlan>,
    join_alias: Option<&Ident>,
) -> proc_macro2::TokenStream {
    let rewritten =
        rewrite_index_join_item_expr(expr, join_alias).unwrap_or_else(|| quote! { #expr });
    let rewritten = syn::parse2(rewritten).expect("rewritten expression remains valid");
    rewrite_row_expr(&rewritten, nested_generator)
}

fn rewrite_index_join_item_expr(
    expr: &Expr,
    join_alias: Option<&Ident>,
) -> Option<proc_macro2::TokenStream> {
    match expr {
        Expr::Field(attribute) => {
            parse_index_join_binding_expr(attribute, join_alias).or_else(|| {
                let base = rewrite_index_join_item_expr(&attribute.base, join_alias)?;
                let member = clone_member(&attribute.member);
                Some(quote! { #base.#member })
            })
        }
        Expr::Binary(binary) => {
            let left = rewrite_index_join_item_expr(&binary.left, join_alias);
            let right = rewrite_index_join_item_expr(&binary.right, join_alias);
            (left.is_some() || right.is_some()).then(|| {
                let left = left.unwrap_or_else(|| {
                    let left = &binary.left;
                    quote! { #left }
                });
                let right = right.unwrap_or_else(|| {
                    let right = &binary.right;
                    quote! { #right }
                });
                let op = &binary.op;
                quote! { #left #op #right }
            })
        }
        Expr::Paren(paren) => {
            rewrite_index_join_item_expr(&paren.expr, join_alias).map(|expr| quote! { (#expr) })
        }
        Expr::Reference(reference) => {
            rewrite_index_join_item_expr(&reference.expr, join_alias).map(|expr| quote! { &#expr })
        }
        Expr::Unary(unary) => {
            let expr = rewrite_index_join_item_expr(&unary.expr, join_alias)?;
            let op = &unary.op;
            Some(quote! { #op #expr })
        }
        Expr::Group(group) => rewrite_index_join_item_expr(&group.expr, join_alias),
        _ => None,
    }
}

fn parse_index_join_binding_expr(
    attribute: &ExprField,
    join_alias: Option<&Ident>,
) -> Option<proc_macro2::TokenStream> {
    let Expr::Path(path) = &*attribute.base else {
        return None;
    };
    if path.qself.is_some()
        || !(path.path.is_ident("join")
            || join_alias.is_some_and(|alias| path.path.is_ident(alias)))
    {
        return None;
    }
    match &attribute.member {
        Member::Unnamed(index) if index.index == 0 => Some(quote! { generator_item.0 }),
        Member::Unnamed(index) if index.index == 1 => Some(quote! { *join_item }),
        _ => None,
    }
}

fn select_join_for_expr<'a>(expr: &Expr, joins: &'a [JoinSpec]) -> Option<&'a JoinSpec> {
    select_join_for_expr_index(expr, joins).map(|(_, join)| join)
}

pub(crate) fn select_join_for_expr_index<'a>(
    expr: &Expr,
    joins: &'a [JoinSpec],
) -> Option<(usize, &'a JoinSpec)> {
    match joins {
        [join] => Some((0, join)),
        joins => joins.iter().enumerate().find(|(_, join)| {
            join.alias
                .as_ref()
                .is_some_and(|alias| expr_mentions_ident(expr, alias))
        }),
    }
}

fn expr_mentions_ident(expr: &Expr, ident: &Ident) -> bool {
    match expr {
        Expr::Path(path) => path.qself.is_none() && path.path.is_ident(ident),
        Expr::Field(attribute) => expr_mentions_ident(&attribute.base, ident),
        Expr::Index(index) => {
            expr_mentions_ident(&index.expr, ident) || expr_mentions_ident(&index.index, ident)
        }
        Expr::Binary(binary) => {
            expr_mentions_ident(&binary.left, ident) || expr_mentions_ident(&binary.right, ident)
        }
        Expr::Call(call) => {
            expr_mentions_ident(&call.func, ident)
                || call.args.iter().any(|arg| expr_mentions_ident(arg, ident))
        }
        Expr::MethodCall(method_call) => {
            expr_mentions_ident(&method_call.receiver, ident)
                || method_call
                    .args
                    .iter()
                    .any(|arg| expr_mentions_ident(arg, ident))
        }
        Expr::Paren(paren) => expr_mentions_ident(&paren.expr, ident),
        Expr::Reference(reference) => expr_mentions_ident(&reference.expr, ident),
        Expr::Unary(unary) => expr_mentions_ident(&unary.expr, ident),
        Expr::Group(group) => expr_mentions_ident(&group.expr, ident),
        _ => false,
    }
}

fn rewrite_join_context_expr(
    expr: &Expr,
    nested_generator: Option<&NestedAxisPlan>,
    join_axis: &Expr,
    join_alias: Option<&Ident>,
) -> proc_macro2::TokenStream {
    let rewritten =
        rewrite_join_item_expr(expr, join_axis, join_alias).unwrap_or_else(|| quote! { #expr });
    let rewritten = syn::parse2(rewritten).expect("rewritten expression remains valid");
    rewrite_row_expr(&rewritten, nested_generator)
}

fn parse_join_axis_expr(expr: &Expr) -> Option<Expr> {
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

fn rewrite_join_item_expr(
    expr: &Expr,
    join_axis: &Expr,
    join_alias: Option<&Ident>,
) -> Option<proc_macro2::TokenStream> {
    match expr {
        Expr::Field(attribute) => parse_join_binding_member_expr(attribute, join_alias)
            .or_else(|| parse_join_member_expr(attribute, join_axis))
            .map(|member| quote! { join_item.#member })
            .or_else(|| {
                let base = rewrite_join_item_expr(&attribute.base, join_axis, join_alias)?;
                let member = clone_member(&attribute.member);
                Some(quote! { #base.#member })
            }),
        Expr::Binary(binary) => {
            let left = rewrite_join_item_expr(&binary.left, join_axis, join_alias);
            let right = rewrite_join_item_expr(&binary.right, join_axis, join_alias);
            (left.is_some() || right.is_some()).then(|| {
                let left = left.unwrap_or_else(|| {
                    let left = &binary.left;
                    quote! { #left }
                });
                let right = right.unwrap_or_else(|| {
                    let right = &binary.right;
                    quote! { #right }
                });
                let op = &binary.op;
                quote! { #left #op #right }
            })
        }
        Expr::Paren(paren) => rewrite_join_item_expr(&paren.expr, join_axis, join_alias)
            .map(|expr| quote! { (#expr) }),
        Expr::Reference(reference) => {
            rewrite_join_item_expr(&reference.expr, join_axis, join_alias)
                .map(|expr| quote! { &#expr })
        }
        Expr::Unary(unary) => {
            let expr = rewrite_join_item_expr(&unary.expr, join_axis, join_alias)?;
            let op = &unary.op;
            Some(quote! { #op #expr })
        }
        Expr::Group(group) => rewrite_join_item_expr(&group.expr, join_axis, join_alias),
        _ => None,
    }
}

fn parse_join_binding_member_expr(
    attribute: &ExprField,
    join_alias: Option<&Ident>,
) -> Option<Member> {
    let Expr::Path(path) = &*attribute.base else {
        return None;
    };
    (path.qself.is_none()
        && (path.path.is_ident("join")
            || join_alias.is_some_and(|alias| path.path.is_ident(alias))))
    .then(|| clone_member(&attribute.member))
}

fn parse_join_member_expr(attribute: &ExprField, join_axis: &Expr) -> Option<Member> {
    let index = field_range_index(attribute)?;
    let requested = rewrite_source_expr(&index.expr, ROOT_ATTR, quote! { self });
    let expected = rewrite_source_expr(join_axis, ROOT_ATTR, quote! { self });
    (requested.to_string() == expected.to_string()).then(|| clone_member(&attribute.member))
}

fn rewrite_context_expr(
    expr: &Expr,
    replacements: &[(&str, proc_macro2::TokenStream)],
) -> proc_macro2::TokenStream {
    let mut rewritten = expr.clone();
    let mut changed = false;

    for (base_name, replacement) in replacements {
        if let Some(next) = rewrite_expr(&rewritten, base_name, replacement) {
            rewritten = next;
            changed = true;
        }
    }

    if changed {
        quote! { #rewritten }
    } else {
        quote! { #expr }
    }
}

fn rewrite_expr(
    expr: &Expr,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    match expr {
        Expr::Path(path) => rewrite_expr_path(path, base_name, replacement),
        Expr::Field(attribute) => rewrite_expr_field(attribute, base_name, replacement),
        Expr::Index(index) => rewrite_expr_index(index, base_name, replacement),
        Expr::Binary(binary) => rewrite_expr_binary(binary, base_name, replacement),
        Expr::Call(call) => rewrite_expr_call(call, base_name, replacement),
        Expr::Closure(closure) => rewrite_expr_closure(closure, base_name, replacement),
        Expr::MethodCall(method_call) => {
            rewrite_expr_method_call(method_call, base_name, replacement)
        }
        Expr::Paren(paren) => rewrite_expr_paren(paren, base_name, replacement),
        Expr::Reference(reference) => rewrite_expr_reference(reference, base_name, replacement),
        Expr::Unary(unary) => rewrite_expr_unary(unary, base_name, replacement),
        Expr::Group(group) => rewrite_expr_group(group, base_name, replacement),
        _ => None,
    }
}

fn rewrite_expr_binary(
    binary: &ExprBinary,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let left = rewrite_expr(&binary.left, base_name, replacement);
    let right = rewrite_expr(&binary.right, base_name, replacement);
    if left.is_none() && right.is_none() {
        return None;
    }
    Some(Expr::Binary(ExprBinary {
        attrs: binary.attrs.clone(),
        left: Box::new(left.unwrap_or_else(|| (*binary.left).clone())),
        op: binary.op,
        right: Box::new(right.unwrap_or_else(|| (*binary.right).clone())),
    }))
}

fn rewrite_expr_paren(
    paren: &ExprParen,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let expr = rewrite_expr(&paren.expr, base_name, replacement)?;
    Some(Expr::Paren(ExprParen {
        attrs: paren.attrs.clone(),
        paren_token: paren.paren_token,
        expr: Box::new(expr),
    }))
}

fn rewrite_expr_method_call(
    method_call: &ExprMethodCall,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let receiver = rewrite_expr(&method_call.receiver, base_name, replacement);
    let mut changed = receiver.is_some();
    let args = method_call
        .args
        .iter()
        .map(|arg| {
            let next = rewrite_expr(arg, base_name, replacement);
            changed |= next.is_some();
            next.unwrap_or_else(|| arg.clone())
        })
        .collect();
    if !changed {
        return None;
    }
    Some(Expr::MethodCall(ExprMethodCall {
        attrs: method_call.attrs.clone(),
        receiver: Box::new(receiver.unwrap_or_else(|| (*method_call.receiver).clone())),
        dot_token: method_call.dot_token,
        method: Ident::new(&method_call.method.to_string(), method_call.method.span()),
        turbofish: method_call.turbofish.clone(),
        paren_token: method_call.paren_token,
        args,
    }))
}

fn rewrite_expr_call(
    call: &ExprCall,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let func = rewrite_expr(&call.func, base_name, replacement);
    let mut changed = func.is_some();
    let args = call
        .args
        .iter()
        .map(|arg| {
            let next = rewrite_expr(arg, base_name, replacement);
            changed |= next.is_some();
            next.unwrap_or_else(|| arg.clone())
        })
        .collect();
    if !changed {
        return None;
    }
    Some(Expr::Call(ExprCall {
        attrs: call.attrs.clone(),
        func: Box::new(func.unwrap_or_else(|| (*call.func).clone())),
        paren_token: call.paren_token,
        args,
    }))
}

fn rewrite_expr_closure(
    closure: &ExprClosure,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let body = rewrite_expr(&closure.body, base_name, replacement)?;
    Some(Expr::Closure(ExprClosure {
        attrs: closure.attrs.clone(),
        lifetimes: closure.lifetimes.clone(),
        constness: closure.constness,
        movability: closure.movability,
        asyncness: closure.asyncness,
        capture: closure.capture,
        or1_token: closure.or1_token,
        inputs: closure.inputs.clone(),
        or2_token: closure.or2_token,
        output: closure.output.clone(),
        body: Box::new(body),
    }))
}

fn rewrite_expr_unary(
    unary: &ExprUnary,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let expr = rewrite_expr(&unary.expr, base_name, replacement)?;
    Some(Expr::Unary(ExprUnary {
        attrs: unary.attrs.clone(),
        op: unary.op,
        expr: Box::new(expr),
    }))
}

fn rewrite_expr_reference(
    reference: &ExprReference,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let expr = rewrite_expr(&reference.expr, base_name, replacement)?;
    Some(Expr::Reference(ExprReference {
        attrs: reference.attrs.clone(),
        and_token: reference.and_token,
        mutability: reference.mutability,
        expr: Box::new(expr),
    }))
}

fn rewrite_expr_group(
    group: &ExprGroup,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let expr = rewrite_expr(&group.expr, base_name, replacement)?;
    Some(Expr::Group(ExprGroup {
        attrs: group.attrs.clone(),
        group_token: group.group_token,
        expr: Box::new(expr),
    }))
}

fn rewrite_expr_path(
    path: &ExprPath,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    if path.qself.is_none() && path.path.is_ident(base_name) {
        Some(syn::parse2(replacement.clone()).expect("replacement is a valid expr"))
    } else {
        None
    }
}

fn rewrite_expr_field(
    attribute: &ExprField,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let base = rewrite_expr(&attribute.base, base_name, replacement)?;
    Some(Expr::Field(ExprField {
        attrs: attribute.attrs.clone(),
        base: Box::new(base),
        dot_token: attribute.dot_token,
        member: clone_member(&attribute.member),
    }))
}

fn rewrite_expr_index(
    index: &ExprIndex,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let base = rewrite_expr(&index.expr, base_name, replacement)?;
    let slice = matches!(&*index.index, Expr::Range(_));
    if !slice {
        return None;
    }
    Some(base)
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
