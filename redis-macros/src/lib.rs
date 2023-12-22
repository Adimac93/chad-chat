use proc_macro::TokenStream;
use proc_macro2::Group;
use quote::ToTokens;
use syn::{FnArg, ItemFn, LitStr, Token, parse_macro_input, Stmt, parse_quote, punctuated::Pair, punctuated::Punctuated, parse::Parser};

#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut fixtures: Punctuated<LitStr, Token![,]> = Punctuated::new();
    
    let meta_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("fixtures") {
            let group: Group = meta.input.parse().unwrap();
            let stream = group.stream().into();
            let parser = Punctuated::<LitStr, Token![,]>::parse_terminated;
            fixtures = parser.parse(stream).unwrap();
            Ok(())
        } else {
            Err(meta.error("expected fixtures"))
        }
    });
    
    parse_macro_input!(attr with meta_parser);
    let mut function = parse_macro_input!(item as ItemFn);

    let Some(FnArg::Typed(last_param)) = function.sig.inputs.pop().map(Pair::into_value) else {
        panic!("Expected a parameter of type redis::aio::ConnectionManager");
    };
    let param = last_param.pat;

    let add_redis_call: Stmt = parse_quote!(let mut #param = redis_macros_core::add_redis::<Vec<&str>>(vec![#fixtures]).await;);
    function.block.stmts.insert(0, add_redis_call);

    function.into_token_stream().into()
}
