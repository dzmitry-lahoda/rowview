use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Expr, ExprField, ExprIndex, ExprPath, Ident, Member, Result, Token, Visibility, braced, parse_macro_input};

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
        if key != "root" {
            return Err(syn::Error::new(key.span(), "expected `root = TypeName`"));
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
    axis: Ident,
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
            if attr.path().is_ident("rowset") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        rowset_name = Some(meta.value()?.parse()?);
                        return Ok(());
                    }
                    if meta.path.is_ident("axis") {
                        axis = Some(meta.value()?.parse()?);
                        return Ok(());
                    }
                    Err(meta.error("unsupported rowset attribute"))
                })?;
            }
        }

        Ok(Self {
            rowset_name: rowset_name
                .ok_or_else(|| syn::Error::new(struct_name.span(), "missing `name`"))?,
            axis: axis.ok_or_else(|| syn::Error::new(struct_name.span(), "missing `axis`"))?,
            struct_name,
            fields,
        })
    }
}

enum FieldKind {
    Copy,
    FromAxis,
}

struct FieldSpec {
    kind: FieldKind,
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
        let mut expr = None;
        let name_span = name.span();
        for attr in attrs {
            if attr.path().is_ident("copy") {
                kind = Some(FieldKind::Copy);
                expr = Some(attr.parse_args()?);
            }
            if attr.path().is_ident("from_axis") {
                kind = Some(FieldKind::FromAxis);
                expr = Some(attr.parse_args()?);
            }
        }

        let _ = ty;

        Ok(Self {
            kind: kind.ok_or_else(|| syn::Error::new(name.span(), "missing field attribute"))?,
            name,
            ty,
            expr: expr.ok_or_else(|| syn::Error::new(name_span, "missing source expression"))?,
        })
    }
}

fn expand_rows(args: RowsArgs, module: RowsModule) -> proc_macro2::TokenStream {
    let root = args.root;
    let module_vis = module.vis;
    let module_name = module.name;
    let rows_type = format_ident!("{}Rows", to_pascal_case(&module_name.to_string()));

    let row_structs = module.rowsets.iter().map(|rowset| {
        let struct_name = &rowset.struct_name;
        let field_defs = rowset.fields.iter().map(|field| {
            let name = &field.name;
            let ty = &field.ty;
            quote! { pub #name: #ty }
        });
        quote! {
            #[derive(Clone, Debug, PartialEq, Eq)]
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
        let axis = &rowset.axis;
        let struct_name = &rowset.struct_name;
        let qualified_struct_name = quote! { #module_name::#struct_name };
        let field_inits = rowset.fields.iter().map(|field| {
            let name = &field.name;
            let value = match field.kind {
                FieldKind::Copy => rewrite_source_expr(&field.expr, "root", quote! { self }),
                FieldKind::FromAxis => rewrite_source_expr(&field.expr, "axis", quote! { axis_item }),
            };
            quote! { #name: #value }
        });

        quote! {
            #rowset_name: self.#axis.iter().map(|axis_item| {
                #qualified_struct_name {
                    #( #field_inits, )*
                }
            }).collect()
        }
    });

    quote! {
        #module_vis mod #module_name {
            #( #row_structs )*

            #[derive(Clone, Debug, PartialEq, Eq)]
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

fn to_pascal_case(name: &str) -> String {
    let mut out = String::new();
    let mut uppercase = true;
    for ch in name.chars() {
        if ch == '_' {
            uppercase = true;
            continue;
        }
        if uppercase {
            for up in ch.to_uppercase() {
                out.push(up);
            }
            uppercase = false;
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        "Rows".to_string()
    } else {
        out
    }
}

fn rewrite_source_expr(
    expr: &Expr,
    base_name: &str,
    replacement: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if let Some(rewritten) = rewrite_expr(expr, base_name, &replacement) {
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
        _ => None,
    }
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
