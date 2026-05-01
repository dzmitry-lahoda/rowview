mod docs;
mod generate;
mod oql_row;
mod parse;
mod schema;
mod solve;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn rows(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as schema::RowViewArgs);
    let module = parse_macro_input!(input as schema::SchemaModule);

    parse::validate_rows(args, module)
        .and_then(generate::expand_rows)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn OqlRow(args: TokenStream, input: TokenStream) -> TokenStream {
    oql_row::expand(args.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
