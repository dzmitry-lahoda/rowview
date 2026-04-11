mod docs;
mod patterns;

use crate::docs::{
    AXIS_ATTR, FieldKind, FieldMode, INCREMENT_BINDING_PREFIX, NAME_ATTR, ROOT_ATTR, ROWS_SUFFIX,
    ROWSET_ATTR,
};
use crate::patterns::{
    FieldSpec, IncrementExpr, JoinOptionSpec, NestedAxisSpec, RowsArgs, RowsModule, RowsetSpec,
};
use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{
    Expr, ExprBinary, ExprCall, ExprClosure, ExprField, ExprGroup, ExprIndex, ExprMethodCall,
    ExprParen, ExprPath, ExprReference, ExprUnary, Ident, Item, ItemStruct, Member, Result, Token,
    braced, parse_macro_input,
};

#[proc_macro_attribute]
pub fn rows(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as RowsArgs);
    let module = parse_macro_input!(input as RowsModule);

    expand_rows(args, module)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

impl Parse for RowsArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key: Ident = input.parse()?;
        if key != ROOT_ATTR {
            return Err(syn::Error::new(
                key.span(),
                "expected `${ROOT_ATTR} = Ident`",
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
            let mut join = None;
            for attr in attrs {
                if attr.path().is_ident(FieldKind::Copy.as_str()) {
                    kind = Some(FieldKind::Copy);
                    if let Ok(named) = attr.parse_args::<IncrementExpr>() {
                        mode = FieldMode::Increment;
                        expr = Some(named.expr);
                    } else {
                        expr = Some(attr.parse_args()?);
                    }
                }
                if attr.path().is_ident(FieldKind::FromAxis.as_str()) {
                    kind = Some(FieldKind::FromAxis);
                    expr = Some(attr.parse_args()?);
                }
                if attr.path().is_ident(FieldKind::FromIndex.as_str()) {
                    kind = Some(FieldKind::FromIndex);
                    expr = Some(attr.parse_args()?);
                }
                if attr.path().is_ident(FieldKind::Join.as_str()) {
                    kind = Some(FieldKind::Join);
                    let spec = attr.parse_args::<JoinOptionSpec>()?;
                    expr = Some(spec.value.clone().ok_or_else(|| {
                        syn::Error::new(name_span, "missing join projection (`select = ...`)")
                    })?);
                    join = Some(spec);
                }
                if attr.path().is_ident(FieldKind::Select.as_str()) {
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
            rowset_name: rowset_name
                .ok_or_else(|| syn::Error::new(struct_name.span(), "missing `${NAME_ATTR}`"))?,
            axis: axis
                .ok_or_else(|| syn::Error::new(struct_name.span(), "missing `${AXIS_ATTR}`"))?,
            struct_name,
            fields,
        })
    }
}

impl Parse for IncrementExpr {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key: Ident = input.parse()?;
        if key != FieldMode::Increment.as_str() {
            return Err(syn::Error::new(
                key.span(),
                format!("expected `{}` = expr", FieldMode::Increment.as_str()),
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
            let key_span = input.span();
            match key.as_str() {
                "left" | "from" => source = Some(input.parse()?),
                "index" => {
                    by_index = true;
                    source = Some(input.parse()?);
                }
                "as" | "alias" => alias = Some(input.parse()?),
                "option" | "on" => condition = Some(input.parse()?),
                "value" | "select" => value = Some(input.parse()?),
                _ if source.is_none() => {
                    alias.get_or_insert_with(|| Ident::new(&key, key_span));
                    source = Some(input.parse()?);
                }
                _ => return Err(input.error(
                    "expected `left`, `from`, `as`, `alias`, `on`, `select`, `option`, or `value`",
                )),
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
            value,
        })
    }
}

fn parse_join_key(input: ParseStream<'_>) -> Result<String> {
    if input.peek(Token![as]) {
        input.parse::<Token![as]>()?;
        Ok("as".to_string())
    } else {
        Ok(input.parse::<Ident>()?.to_string())
    }
}

fn expand_rows(args: RowsArgs, module: RowsModule) -> Result<proc_macro2::TokenStream> {
    let root = args.root;
    let module_vis = module.vis;
    let module_name = module.name;
    let module_imports = module.imports;
    let rows_type = format_ident!(
        "{}{ROWS_SUFFIX}",
        module_name.to_string().to_upper_camel_case()
    );

    let row_structs = module.rowsets.iter().map(|rowset| {
        let attrs = &rowset.attrs;
        let struct_name = &rowset.struct_name;
        let field_defs = rowset.fields.iter().map(|field| {
            let name = &field.name;
            let ty = &field.ty;
            quote! { pub #name: #ty }
        });
        quote! {
            #( #attrs )*
            #[derive(Clone, Debug, PartialEq)]
            pub struct #struct_name {
                #( #field_defs, )*
            }
        }
    });

    let rows_fields = module.rowsets.iter().map(|rowset| {
        let rowset_name = &rowset.rowset_name;
        let struct_name = &rowset.struct_name;
        quote! { pub #rowset_name: ::std::vec::Vec<#struct_name> }
    });

    let builders = module.rowsets.iter().map(|rowset| -> Result<_> {
        let rowset_name = &rowset.rowset_name;
        let nested_axis = parse_nested_axis_expr(&rowset.axis);
        let axis_iter = rewrite_axis_iter_expr(&rowset.axis, nested_axis.as_ref());
        let rowset_joins = &rowset.joins;
        let index_join_len_asserts = rowset_joins
            .iter()
            .filter(|join| join.by_index)
            .map(|join| -> Result<_> {
                let join_source = join.source.as_ref().ok_or_else(|| {
                    syn::Error::new(Span::call_site(), "index join requires source")
                })?;
                let axis = rewrite_source_expr(&rowset.axis, ROOT_ATTR, quote! { self });
                let join_source = rewrite_source_expr(join_source, ROOT_ATTR, quote! { self });
                Ok(quote! {
                    assert_eq!(
                        (#axis).len(),
                        (#join_source).len(),
                        "rowview index join requires axis and joined collection lengths to match"
                    );
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let struct_name = &rowset.struct_name;
        let qualified_struct_name = quote! { #module_name::#struct_name };
        let increment_bindings = rowset.fields.iter().filter_map(|field| {
            if !matches!(field.mode, FieldMode::Increment) {
                return None;
            }
            let binding = format_ident!("{INCREMENT_BINDING_PREFIX}{}", field.name);
            let value = rewrite_row_expr(&field.expr, nested_axis.as_ref());
            Some(quote! {
                let mut #binding = #value;
            })
        });
        let field_inits = rowset.fields.iter().map(|field| -> Result<_> {
            let name = &field.name;
            let value = match (&field.kind, &field.mode) {
                (FieldKind::Copy, FieldMode::Direct) | (FieldKind::FromAxis, FieldMode::Direct) => {
                    rewrite_row_expr(&field.expr, nested_axis.as_ref())
                }
                (FieldKind::FromIndex, FieldMode::Direct) => {
                    quote! {
                        axis_index
                            .try_into()
                            .expect("rowview axis index exceeds target field type capacity")
                    }
                }
                (FieldKind::Join, FieldMode::Direct) => {
                    rewrite_join_expr(field.join.as_ref().expect("join field has spec"), nested_axis.as_ref())?
                }
                (FieldKind::Select, FieldMode::Direct) => {
                    let join = select_join_for_expr(&field.expr, rowset_joins)
                        .ok_or_else(|| syn::Error::new_spanned(&field.expr, "select field requires a matching row-level `#[joins(...)]`"))?;
                    rewrite_join_select_expr(join, &field.expr, nested_axis.as_ref())?
                }
                (FieldKind::Copy, FieldMode::Increment) => {
                    let binding = format_ident!("{INCREMENT_BINDING_PREFIX}{}", field.name);
                    quote! {{
                        let value = #binding;
                        #binding += 1;
                        value
                    }}
                }
                (FieldKind::FromAxis | FieldKind::FromIndex | FieldKind::Join | FieldKind::Select, FieldMode::Increment) => unreachable!(),
            };
            Ok(quote! { #name: #value })
        }).collect::<Result<Vec<_>>>()?;

        let row_values = if matches!(&rowset.axis, Expr::Tuple(tuple) if tuple.elems.is_empty()) {
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
                    #( #increment_bindings )*
                    #axis_iter.enumerate().map(|(axis_index, axis_item)| {
                        let (axis_parent, axis_item) = axis_item;
                        #qualified_struct_name {
                            #( #field_inits, )*
                        }
                    }).collect()
                }
            }
        };

        Ok(quote! {
            #rowset_name: #row_values
        })
    }).collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #module_vis mod #module_name {
            #( #module_imports )*
            #( #row_structs )*

            #[derive(Clone, Debug, PartialEq)]
            pub struct #rows_type {
                #( #rows_fields, )*
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
    nested_axis: Option<&NestedAxisSpec>,
) -> proc_macro2::TokenStream {
    if let Some(iter) = rewrite_nested_axis_iter_expr(nested_axis) {
        iter
    } else {
        let axis = rewrite_source_expr(expr, ROOT_ATTR, quote! { self });
        quote! { (#axis).iter().map(|axis_item| { ((), axis_item) }) }
    }
}

fn parse_nested_axis_expr(expr: &Expr) -> Option<NestedAxisSpec> {
    let Expr::Field(field) = expr else {
        return None;
    };
    let index = field_range_index(field)?;

    Some(NestedAxisSpec {
        parent: (*index.expr).clone(),
        child: clone_member(&field.member),
    })
}

fn rewrite_nested_axis_iter_expr(
    nested_axis: Option<&NestedAxisSpec>,
) -> Option<proc_macro2::TokenStream> {
    let nested_axis = nested_axis?;
    let base = rewrite_source_expr(&nested_axis.parent, ROOT_ATTR, quote! { self });
    let member = clone_member(&nested_axis.child);
    Some(quote! {
        (#base).iter().flat_map(|axis_parent| {
            axis_parent.#member.iter().map(move |axis_item| (axis_parent, axis_item))
        })
    })
}

fn rewrite_row_expr(expr: &Expr, nested_axis: Option<&NestedAxisSpec>) -> proc_macro2::TokenStream {
    if let Some(nested_axis) = nested_axis
        && let Some(parent_expr) = rewrite_parent_expr(expr, nested_axis)
    {
        return parent_expr;
    }

    rewrite_context_expr(
        expr,
        &[
            (ROOT_ATTR, quote! { self }),
            (AXIS_ATTR, quote! { axis_item }),
        ],
    )
}

fn rewrite_parent_expr(
    expr: &Expr,
    nested_axis: &NestedAxisSpec,
) -> Option<proc_macro2::TokenStream> {
    let Expr::Field(field) = expr else {
        return None;
    };
    let index = field_range_index(field)?;
    let parent = rewrite_source_expr(&nested_axis.parent, ROOT_ATTR, quote! { self });
    let requested_parent = rewrite_source_expr(&index.expr, ROOT_ATTR, quote! { self });
    if parent.to_string() != requested_parent.to_string() {
        return None;
    }
    let member = clone_member(&field.member);
    Some(quote! { axis_parent.#member })
}

fn field_range_index(field: &ExprField) -> Option<&ExprIndex> {
    let Expr::Index(index) = &*field.base else {
        return None;
    };
    matches!(&*index.index, Expr::Range(_)).then_some(index)
}

fn rewrite_join_expr(
    join: &JoinOptionSpec,
    nested_axis: Option<&NestedAxisSpec>,
) -> Result<proc_macro2::TokenStream> {
    let value = join.value.as_ref().ok_or_else(|| {
        let span_expr = join.condition.as_ref().or(join.source.as_ref());
        span_expr.map_or_else(
            || syn::Error::new(Span::call_site(), "join field requires `select = ...`"),
            |expr| syn::Error::new_spanned(expr, "join field requires `select = ...`"),
        )
    })?;
    rewrite_join_select_expr(join, value, nested_axis)
}

fn rewrite_join_select_expr(
    join: &JoinOptionSpec,
    value_expr: &Expr,
    nested_axis: Option<&NestedAxisSpec>,
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
            "join expression must provide a source like `#[join(root.values[..], on = ..., select = ...)]` or reference a collection slice like `root.values[..].field`",
        ))?;
    let join_iter = rewrite_source_expr(&join_axis, ROOT_ATTR, quote! { self });
    if join.by_index {
        let value = rewrite_index_join_context_expr(value_expr, nested_axis, join.alias.as_ref());
        return Ok(quote! {
            (#join_iter)
                .iter()
                .nth(axis_index)
                .and_then(|join_item| ::core::option::Option::Some(#value))
        });
    }
    let condition_expr = join
        .condition
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(value_expr, "join requires `on = ...`"))?;
    let condition = rewrite_join_context_expr(
        condition_expr,
        nested_axis,
        &join_axis,
        join.alias.as_ref(),
    );
    let value = rewrite_join_context_expr(value_expr, nested_axis, &join_axis, join.alias.as_ref());

    Ok(quote! {
        (#join_iter)
            .iter()
            .find(|join_item| #condition)
            .and_then(|join_item| ::core::option::Option::Some(#value))
    })
}

fn rewrite_index_join_context_expr(
    expr: &Expr,
    nested_axis: Option<&NestedAxisSpec>,
    join_alias: Option<&Ident>,
) -> proc_macro2::TokenStream {
    let rewritten =
        rewrite_index_join_item_expr(expr, join_alias).unwrap_or_else(|| quote! { #expr });
    let rewritten = syn::parse2(rewritten).expect("rewritten expression remains valid");
    rewrite_row_expr(&rewritten, nested_axis)
}

fn rewrite_index_join_item_expr(
    expr: &Expr,
    join_alias: Option<&Ident>,
) -> Option<proc_macro2::TokenStream> {
    match expr {
        Expr::Field(field) => parse_index_join_binding_expr(field, join_alias).or_else(|| {
            let base = rewrite_index_join_item_expr(&field.base, join_alias)?;
            let member = clone_member(&field.member);
            Some(quote! { #base.#member })
        }),
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
    field: &ExprField,
    join_alias: Option<&Ident>,
) -> Option<proc_macro2::TokenStream> {
    let Expr::Path(path) = &*field.base else {
        return None;
    };
    if path.qself.is_some()
        || !(path.path.is_ident("join") || join_alias.is_some_and(|alias| path.path.is_ident(alias)))
    {
        return None;
    }
    match &field.member {
        Member::Unnamed(index) if index.index == 0 => Some(quote! { axis_item.0 }),
        Member::Unnamed(index) if index.index == 1 => Some(quote! { *join_item }),
        _ => None,
    }
}

fn select_join_for_expr<'a>(
    expr: &Expr,
    joins: &'a [JoinOptionSpec],
) -> Option<&'a JoinOptionSpec> {
    match joins {
        [join] => Some(join),
        joins => joins.iter().find(|join| {
            join.alias
                .as_ref()
                .is_some_and(|alias| expr_mentions_ident(expr, alias))
        }),
    }
}

fn expr_mentions_ident(expr: &Expr, ident: &Ident) -> bool {
    match expr {
        Expr::Path(path) => path.qself.is_none() && path.path.is_ident(ident),
        Expr::Field(field) => expr_mentions_ident(&field.base, ident),
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
    nested_axis: Option<&NestedAxisSpec>,
    join_axis: &Expr,
    join_alias: Option<&Ident>,
) -> proc_macro2::TokenStream {
    let rewritten =
        rewrite_join_item_expr(expr, join_axis, join_alias).unwrap_or_else(|| quote! { #expr });
    let rewritten = syn::parse2(rewritten).expect("rewritten expression remains valid");
    rewrite_row_expr(&rewritten, nested_axis)
}

fn parse_join_axis_expr(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Field(field) => parse_join_axis_field_expr(field),
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

fn parse_join_axis_field_expr(field: &ExprField) -> Option<Expr> {
    field_range_index(field)
        .map(|index| (*index.expr).clone())
        .or_else(|| parse_join_axis_expr(&field.base))
}

fn rewrite_join_item_expr(
    expr: &Expr,
    join_axis: &Expr,
    join_alias: Option<&Ident>,
) -> Option<proc_macro2::TokenStream> {
    match expr {
        Expr::Field(field) => parse_join_binding_member_expr(field, join_alias)
            .or_else(|| parse_join_member_expr(field, join_axis))
            .map(|member| quote! { join_item.#member })
            .or_else(|| {
                let base = rewrite_join_item_expr(&field.base, join_axis, join_alias)?;
                let member = clone_member(&field.member);
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

fn parse_join_binding_member_expr(field: &ExprField, join_alias: Option<&Ident>) -> Option<Member> {
    let Expr::Path(path) = &*field.base else {
        return None;
    };
    (path.qself.is_none()
        && (path.path.is_ident("join")
            || join_alias.is_some_and(|alias| path.path.is_ident(alias))))
    .then(|| clone_member(&field.member))
}

fn parse_join_member_expr(field: &ExprField, join_axis: &Expr) -> Option<Member> {
    let index = field_range_index(field)?;
    let requested = rewrite_source_expr(&index.expr, ROOT_ATTR, quote! { self });
    let expected = rewrite_source_expr(join_axis, ROOT_ATTR, quote! { self });
    (requested.to_string() == expected.to_string()).then(|| clone_member(&field.member))
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
        Expr::Field(field) => rewrite_expr_field(field, base_name, replacement),
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
    field: &ExprField,
    base_name: &str,
    replacement: &proc_macro2::TokenStream,
) -> Option<Expr> {
    let base = rewrite_expr(&field.base, base_name, replacement)?;
    Some(Expr::Field(ExprField {
        attrs: field.attrs.clone(),
        base: Box::new(base),
        dot_token: field.dot_token,
        member: clone_member(&field.member),
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
