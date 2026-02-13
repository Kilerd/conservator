use proc_macro::TokenStream;

mod creatable;
mod domain;
mod selectable;
mod sql;
mod text_enum;

#[proc_macro_derive(Domain, attributes(domain))]
pub fn derive_domain_fn(input: TokenStream) -> TokenStream {
    let stream2 = proc_macro2::TokenStream::from(input);
    match domain::handler(stream2) {
        Ok(stream) => proc_macro::TokenStream::from(stream),
        Err((span, msg)) => {
            let error = quote::quote_spanned! {span=>
                compile_error!(#msg);
            };
            proc_macro::TokenStream::from(error)
        }
    }
}

/// 派生 Selectable trait
///
/// 自动生成 `Selectable` 和 `tokio_postgres::Row` 的实现。
/// 用于定义可以作为 `SelectBuilder.returning::<T>()` 目标的投影类型。
///
/// # Example
/// ```ignore
/// #[derive(Selectable)]
/// struct UserSummary {
///     id: i32,
///     name: String,
/// }
/// ```
#[proc_macro_derive(Selectable)]
pub fn derive_selectable_fn(input: TokenStream) -> TokenStream {
    let stream2 = proc_macro2::TokenStream::from(input);
    match selectable::handler(stream2) {
        Ok(stream) => proc_macro::TokenStream::from(stream),
        Err((span, msg)) => {
            let error = quote::quote_spanned! {span=>
                compile_error!(#msg);
            };
            proc_macro::TokenStream::from(error)
        }
    }
}

#[proc_macro_derive(Creatable)]
pub fn derive_creatable_fn(input: TokenStream) -> TokenStream {
    let stream2 = proc_macro2::TokenStream::from(input);
    match creatable::handler(stream2) {
        Ok(stream) => proc_macro::TokenStream::from(stream),
        Err((span, msg)) => {
            let error = quote::quote_spanned! {span=>
                compile_error!(#msg);
            };
            proc_macro::TokenStream::from(error)
        }
    }
}

#[proc_macro_attribute]
pub fn sql(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let stream2 = proc_macro2::TokenStream::from(input);
    match sql::handler(args, stream2) {
        Ok(stream) => proc_macro::TokenStream::from(stream),
        Err((span, msg)) => {
            let error = quote::quote_spanned! {span=>
                compile_error!(#msg);
            };
            proc_macro::TokenStream::from(error)
        }
    }
}

/// Derive `SqlType` for enums stored as TEXT in PostgreSQL.
///
/// Supports `#[serde(rename = "...")]` to customize the string representation.
///
/// # Example
/// ```ignore
/// #[derive(Debug, TextEnum)]
/// enum MessageType {
///     Inbound,
///     Outbound,
/// }
///
/// // With serde rename
/// #[derive(Debug, Serialize, Deserialize, TextEnum)]
/// enum Status {
///     #[serde(rename = "active")]
///     Active,
///     #[serde(rename = "inactive")]
///     Inactive,
/// }
/// ```
#[proc_macro_derive(TextEnum, attributes(serde))]
pub fn derive_text_enum_fn(input: TokenStream) -> TokenStream {
    let stream2 = proc_macro2::TokenStream::from(input);
    match text_enum::handler(stream2) {
        Ok(stream) => proc_macro::TokenStream::from(stream),
        Err((span, msg)) => {
            let error = quote::quote_spanned! {span=>
                compile_error!(#msg);
            };
            proc_macro::TokenStream::from(error)
        }
    }
}
