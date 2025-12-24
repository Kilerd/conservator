use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Fields, parse2};

pub(crate) fn handler(
    input: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream, (Span, &'static str)> {
    let derive_input =
        parse2::<DeriveInput>(input).map_err(|_| (Span::call_site(), "Failed to parse input"))?;

    let ident = &derive_input.ident;

    // 获取结构体字段
    let fields = match &derive_input.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(named) => &named.named,
            _ => return Err((derive_input.ident.span(), "Only named fields are supported")),
        },
        _ => return Err((derive_input.ident.span(), "Only structs are supported")),
    };

    // 收集字段信息
    let field_info: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            field
                .ident
                .as_ref()
                .map(|ident| (ident.clone(), field.ty.clone()))
        })
        .collect();

    // 生成列名数组
    let column_names: Vec<String> = field_info
        .iter()
        .map(|(ident, _)| ident.to_string())
        .collect();

    // 生成 FromRow 的 try_get 调用
    let field_idents: Vec<_> = field_info.iter().map(|(ident, _)| ident).collect();
    let field_names: Vec<String> = field_info
        .iter()
        .map(|(ident, _)| ident.to_string())
        .collect();

    let ret = quote! {
        impl ::conservator::Selectable for #ident {
            const COLUMN_NAMES: &'static [&'static str] = &[#(#column_names),*];

            fn from_row(row: &::tokio_postgres::Row) -> Result<Self, ::conservator::Error> {
                use conservator::SqlTypeWrapper;
                Ok(Self {
                    #(#field_idents: { let wrapper: SqlTypeWrapper<_> = row.try_get(#field_names)?; wrapper.0 }),*
                })
            }
        }
    };

    Ok(ret)
}
