use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Lit, Meta, NestedMeta, parse2};

pub(crate) fn handler(
    input: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream, (Span, &'static str)> {
    let derive_input =
        parse2::<DeriveInput>(input).map_err(|_| (Span::call_site(), "Failed to parse input"))?;

    let ident = &derive_input.ident;

    // Only support enums
    let variants = match &derive_input.data {
        syn::Data::Enum(data_enum) => &data_enum.variants,
        _ => return Err((derive_input.ident.span(), "TextEnum only supports enums")),
    };

    // Extract rename_all from enum-level attributes
    let rename_all = extract_serde_rename_all(&derive_input.attrs);

    // Collect variant info: (variant_ident, string_value)
    let mut variant_info: Vec<(syn::Ident, String)> = Vec::new();

    for variant in variants {
        // Only support unit variants (no fields)
        match &variant.fields {
            syn::Fields::Unit => {}
            _ => {
                return Err((
                    variant.ident.span(),
                    "TextEnum only supports unit variants (no fields)",
                ))
            }
        }

        let variant_ident = variant.ident.clone();

        // Priority: variant-level rename > enum-level rename_all > original name
        let string_value = extract_serde_rename(&variant.attrs).unwrap_or_else(|| {
            let original = variant_ident.to_string();
            match &rename_all {
                Some(rule) => apply_rename_rule(&original, rule),
                None => original,
            }
        });

        variant_info.push((variant_ident, string_value));
    }

    // Generate match arms for to_sql_value
    let to_sql_arms = variant_info.iter().map(|(variant_ident, string_value)| {
        quote! {
            Self::#variant_ident => #string_value
        }
    });

    // Generate match arms for from_sql_value
    let from_sql_arms = variant_info.iter().map(|(variant_ident, string_value)| {
        quote! {
            #string_value => Ok(Self::#variant_ident)
        }
    });

    // Collect all valid string values for error message
    let valid_values: Vec<&str> = variant_info.iter().map(|(_, s)| s.as_str()).collect();
    let valid_values_str = valid_values.join(", ");

    let ret = quote! {
        impl ::conservator::SqlType for #ident {
            fn to_sql_value(
                &self,
                ty: &::tokio_postgres::types::Type,
                out: &mut ::tokio_postgres::types::private::BytesMut,
            ) -> Result<::tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
                let s: &str = match self {
                    #(#to_sql_arms),*
                };
                ::tokio_postgres::types::ToSql::to_sql(&s.to_string(), ty, out)
            }

            fn from_sql_value(
                ty: &::tokio_postgres::types::Type,
                raw: &[u8],
            ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
                let s: String = ::tokio_postgres::types::FromSql::from_sql(ty, raw)?;
                match s.as_str() {
                    #(#from_sql_arms,)*
                    other => Err(format!(
                        "invalid value '{}' for {}, expected one of: {}",
                        other,
                        stringify!(#ident),
                        #valid_values_str
                    ).into())
                }
            }

            fn accepts(ty: &::tokio_postgres::types::Type) -> bool {
                matches!(
                    *ty,
                    ::tokio_postgres::types::Type::TEXT
                        | ::tokio_postgres::types::Type::VARCHAR
                        | ::tokio_postgres::types::Type::BPCHAR
                )
            }
        }
    };

    Ok(ret)
}

/// Extract the rename value from #[serde(rename = "...")] attribute on a variant
fn extract_serde_rename(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path.is_ident("serde") {
            continue;
        }

        if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
            for nested in &meta_list.nested {
                if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested {
                    if name_value.path.is_ident("rename") {
                        if let Lit::Str(lit_str) = &name_value.lit {
                            return Some(lit_str.value());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extract the rename_all value from #[serde(rename_all = "...")] attribute on the enum
fn extract_serde_rename_all(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path.is_ident("serde") {
            continue;
        }

        if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
            for nested in &meta_list.nested {
                if let NestedMeta::Meta(Meta::NameValue(name_value)) = nested {
                    if name_value.path.is_ident("rename_all") {
                        if let Lit::Str(lit_str) = &name_value.lit {
                            return Some(lit_str.value());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Apply a serde rename_all rule to a PascalCase identifier
fn apply_rename_rule(name: &str, rule: &str) -> String {
    match rule {
        "lowercase" => name.to_lowercase(),
        "UPPERCASE" => name.to_uppercase(),
        "PascalCase" => name.to_string(), // Already PascalCase
        "camelCase" => to_camel_case(name),
        "snake_case" => to_snake_case(name),
        "SCREAMING_SNAKE_CASE" => to_snake_case(name).to_uppercase(),
        "kebab-case" => to_snake_case(name).replace('_', "-"),
        "SCREAMING-KEBAB-CASE" => to_snake_case(name).to_uppercase().replace('_', "-"),
        _ => name.to_string(), // Unknown rule, keep original
    }
}

/// Convert PascalCase to snake_case
fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }
    result
}

/// Convert PascalCase to camelCase
fn to_camel_case(name: &str) -> String {
    let mut result = String::new();
    let mut first = true;
    for ch in name.chars() {
        if first {
            result.push(ch.to_lowercase().next().unwrap_or(ch));
            first = false;
        } else {
            result.push(ch);
        }
    }
    result
}
