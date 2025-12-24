use darling::FromDeriveInput;
use itertools::Itertools;
use proc_macro2::Span;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, parse2};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(crud))]
struct CreatableOpts {
    ident: syn::Ident,
}

pub(crate) fn handler(
    input: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream, (Span, String)> {
    let x1 = parse2::<DeriveInput>(input.clone()).map_err(|e| {
        (
            e.span(),
            format!("failed to parse struct definition: {}", e),
        )
    })?;

    let creatable_opts: CreatableOpts = CreatableOpts::from_derive_input(&x1).map_err(|e| {
        (
            x1.span(),
            format!("failed to parse #[creatable] attributes: {}", e),
        )
    })?;

    let ident = creatable_opts.ident;

    // Early return: only structs are supported
    let Data::Struct(ref body) = x1.data else {
        return Err((
            x1.span(),
            "Creatable can only be derived for structs, not enums".to_string(),
        ));
    };

    // Extract field information
    let fields = body.fields.iter().map(|it| &it.ident).collect::<Vec<_>>();

    let field_list = fields
        .iter()
        .map(|it| {
            it.as_ref()
                .map(|ident| format!("\"{}\"", ident))
                .ok_or_else(|| (x1.span(), "field identifier is missing".to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(",");

    let param_list = fields
        .iter()
        .enumerate()
        .map(|it| it.0)
        .map(|it| format!("${}", it + 1))
        .join(",");

    let columns = format!("({})", field_list);
    let insert_sql = format!("({})", param_list);
    let fields_len = fields.len();

    // Generate field value conversions
    let values_list = fields.iter().map(|it| {
        quote! {
            ::conservator::IntoValue::into_value(self.#it.clone())
        }
    });

    Ok(quote! {
        impl ::conservator::Creatable for #ident {

            fn get_columns(&self) -> &str {
                #columns
            }

            fn get_insert_sql(&self) -> &str {
                #insert_sql
            }

            fn get_batch_insert_sql(&self, idx: usize) -> String {
                let mut ret = String::new();
                ret.push_str("(");
                for i in 0..#fields_len {
                    if i > 0 {
                        ret.push_str(",");
                    }
                    ret.push_str(&format!("${}", idx * #fields_len + i + 1));
                }
                ret.push_str(")");
                ret
            }

            fn get_values(&self) -> Vec<::conservator::Value> {
                vec![
                    #(#values_list),*
                ]
            }

            fn get_batch_values(&self, _idx: usize) -> Vec<::conservator::Value> {
                // 批量插入时，每个项目的值计算方式相同
                self.get_values()
            }
        }
    })
}

#[cfg(test)]
mod test {
    #[test]
    fn compile_fail() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/fail/*.rs");
    }
}
