mod docs;

use crate::docs::{AXIS_ATTR, FieldKind, FieldMode, INCREMENT_BINDING_PREFIX, NAME_ATTR, ROOT_ATTR, ROWSET_ATTR, ROWS_SUFFIX};
use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Expr, ExprBinary, ExprField, ExprGroup, ExprIndex, ExprMethodCall, ExprParen, ExprPath, ExprUnary, Ident, Member, Result, Token, Visibility, braced, parse_macro_input};

#[proc_macro_attribute]
pub fn rows(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as RowsArgs);
    let module = parse_macro_input!(input as RowsModule);

    expand_rows(args, module).into()
}

struct RowsArgs {
    root: Ident,
}

impl Parse for RowsArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key: Ident = input.parse()?;
        if key != ROOT_ATTR {
            return Err(syn::Error::new(key.span(), "expected `${ROOT_ATTR} = Ident`"));
        }
        input.parse::<Token![=]>()?;
        Ok(Self {
            root: input.parse()?,
        })
    }
}

struct RowsModule {
    vis: Visibility,
    name: Ident,
    rowsets: Vec<RowsetSpec>,
}

impl Parse for RowsModule {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let vis = input.parse()?;
        input.parse::<Token![mod]>()?;
        let name: Ident = input.parse()?;
        let content;
        braced!(content in input);

        let mut rowsets = Vec::new();
        while !content.is_empty() {
            rowsets.push(content.parse()?);
        }

        Ok(Self { vis, name, rowsets })
    }
}

struct RowsetSpec {
    rowset_name: Ident,
    axis: Expr,
    struct_name: Ident,
    fields: Vec<FieldSpec>,
}

impl Parse for RowsetSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        input.parse::<Token![struct]>()?;
        let struct_name: Ident = input.parse()?;
        let fields_content;
        braced!(fields_content in input);

        let mut fields = Vec::new();
        while !fields_content.is_empty() {
            fields.push(fields_content.parse()?);
        }

        let mut rowset_name = None;
        let mut axis = None;
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
            }
        }

        Ok(Self {
            rowset_name: rowset_name
                .ok_or_else(|| syn::Error::new(struct_name.span(), "missing `${NAME_ATTR}`"))?,
            axis: axis.ok_or_else(|| syn::Error::new(struct_name.span(), "missing `${AXIS_ATTR}`"))?,
            struct_name,
            fields,
        })
    }
}


struct FieldSpec {
    kind: FieldKind,
    mode: FieldMode,
    name: Ident,
    ty: syn::Type,
    expr: Expr,
}

impl Parse for FieldSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;

        let ty: syn::Type = input.parse()?;
        input.parse::<Token![,]>()?;

        let mut kind = None;
        let mut mode = FieldMode::Direct;
        let mut expr = None;
        let name_span = name.span();
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
        }

        let _ = ty;

        Ok(Self {
            kind: kind.ok_or_else(|| syn::Error::new(name.span(), "missing field attribute"))?,
            mode,
            name,
            ty,
            expr: expr.ok_or_else(|| syn::Error::new(name_span, "missing source expression"))?,
        })
    }
}

struct IncrementExpr {
    expr: Expr,
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
        Ok(Self { expr: input.parse()? })
    }
}

fn expand_rows(args: RowsArgs, module: RowsModule) -> proc_macro2::TokenStream {
    let root = args.root;
    let module_vis = module.vis;
    let module_name = module.name;
    let rows_type = format_ident!("{}{ROWS_SUFFIX}", module_name.to_string().to_upper_camel_case());

    let row_structs = module.rowsets.iter().map(|rowset| {
        let struct_name = &rowset.struct_name;
        let field_defs = rowset.fields.iter().map(|field| {
            let name = &field.name;
            let ty = &field.ty;
            quote! { pub #name: #ty }
        });
        quote! {
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

    let builders = module.rowsets.iter().map(|rowset| {
        let rowset_name = &rowset.rowset_name;
        let axis = rewrite_source_expr(&rowset.axis, ROOT_ATTR, quote! { self });
        let struct_name = &rowset.struct_name;
        let qualified_struct_name = quote! { #module_name::#struct_name };
        let increment_bindings = rowset.fields.iter().filter_map(|field| {
            if !matches!(field.mode, FieldMode::Increment) {
                return None;
            }
            let binding = format_ident!("{INCREMENT_BINDING_PREFIX}{}", field.name);
            let value = rewrite_context_expr(
                &field.expr,
                &[
                    (ROOT_ATTR, quote! { self }),
                    (AXIS_ATTR, quote! { axis_item }),
                ],
            );
            Some(quote! {
                let mut #binding = #value;
            })
        });
        let field_inits = rowset.fields.iter().map(|field| {
            let name = &field.name;
            let value = match (&field.kind, &field.mode) {
                (FieldKind::Copy, FieldMode::Direct) | (FieldKind::FromAxis, FieldMode::Direct) => rewrite_context_expr(
                    &field.expr,
                    &[
                        (ROOT_ATTR, quote! { self }),
                        (AXIS_ATTR, quote! { axis_item }),
                    ],
                ),
                (FieldKind::Copy, FieldMode::Increment) => {
                    let binding = format_ident!("{INCREMENT_BINDING_PREFIX}{}", field.name);
                    quote! {{
                        let value = #binding;
                        #binding += 1;
                        value
                    }}
                }
                (FieldKind::FromAxis, FieldMode::Increment) => unreachable!(),
            };
            quote! { #name: #value }
        });

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
                    #( #increment_bindings )*
                    (#axis).iter().map(|axis_item| {
                        #qualified_struct_name {
                            #( #field_inits, )*
                        }
                    }).collect()
                }
            }
        };

        quote! {
            #rowset_name: #row_values
        }
    });

    quote! {
        #module_vis mod #module_name {
            #( #row_structs )*

            #[derive(Clone, Debug, PartialEq)]
            pub struct #rows_type {
                #( #rows_fields, )*
            }
        }

        impl #root {
            pub fn to_rows(&self) -> #module_name::#rows_type {
                #module_name::#rows_type {
                    #( #builders, )*
                }
            }
        }
    }
}

fn rewrite_source_expr(
    expr: &Expr,
    base_name: &str,
    replacement: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    rewrite_context_expr(expr, &[(base_name, replacement)])
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
        Expr::MethodCall(method_call) => rewrite_expr_method_call(method_call, base_name, replacement),
        Expr::Paren(paren) => rewrite_expr_paren(paren, base_name, replacement),
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
        op: binary.op.clone(),
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
    let receiver = rewrite_expr(&method_call.receiver, base_name, replacement)?;
    Some(Expr::MethodCall(ExprMethodCall {
        attrs: method_call.attrs.clone(),
        receiver: Box::new(receiver),
        dot_token: method_call.dot_token,
        method: Ident::new(&method_call.method.to_string(), method_call.method.span()),
        turbofish: method_call.turbofish.clone(),
        paren_token: method_call.paren_token,
        args: method_call.args.clone(),
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
        op: unary.op.clone(),
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
