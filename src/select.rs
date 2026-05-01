use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, Result};

pub(crate) fn expand(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "`#[rowview::Select]` does not accept arguments",
        ));
    }

    let item: ItemStruct = syn::parse2(input)?;

    Ok(quote! {
        #[derive(::layout::SOA, ::derive_more::From)]
        #item
    })
}
