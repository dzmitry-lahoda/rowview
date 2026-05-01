#[path = "../../src/docs.rs"]
mod docs;
#[path = "../../src/generate.rs"]
mod generate;
#[path = "../../src/parse.rs"]
mod parse;
#[path = "../../src/schema.rs"]
mod schema;
#[path = "../../src/solve.rs"]
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
