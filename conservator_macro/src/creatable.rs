use darling::FromDeriveInput;
use itertools::Itertools;
use proc_macro_error::abort;
use quote::quote;
use syn::{parse2, Data, DeriveInput};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(crud))]
struct CreatableOpts {
    ident: syn::Ident,
}

pub(crate) fn handle_creatable(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let x1 = parse2::<DeriveInput>(input).unwrap();
    let creatable_opts: CreatableOpts = CreatableOpts::from_derive_input(&x1).unwrap();

    let ident = creatable_opts.ident;

    if let Data::Struct(ref body) = x1.data {
        let fields = body.fields.iter().map(|it| &it.ident).collect::<Vec<_>>();

        let field_list = fields
            .iter()
            .map(|it| {
                format!(
                    "\"{}\"",
                    it.as_ref()
                        .map(|ident| ident.to_string())
                        .expect("ident not found")
                )
            })
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

        let bind_list_for_query_as = fields.iter().map(|it| {
            quote! { .bind(self. #it)}
        });
        let bind_list_for_query = fields.iter().map(|it| {
            quote! { .bind(self. #it)}
        });
        let bind_list_for_query_scalar = fields.iter().map(|it| {
            quote! { .bind(self. #it)}
        });

        quote! {
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
                fn build_for_query_as<'q, O>(
                    self,
                    e: ::sqlx::query::QueryAs<'q, ::sqlx::Postgres, O, <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments,>,
                ) -> ::sqlx::query::QueryAs<'q, ::sqlx::Postgres, O, <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments,> {
                    e
                    #(#bind_list_for_query_as)*
                }
                fn build_for_query<'q>(
                    self,
                    e: ::sqlx::query::Query<'q, ::sqlx::Postgres, <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments>,
                ) -> ::sqlx::query::Query<'q, ::sqlx::Postgres, <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments> {
                    e
                    #(#bind_list_for_query)*
                }
                fn bind_to_query_scalar<'q, O>(
                    self,
                    e: ::sqlx::query::QueryScalar<'q, ::sqlx::Postgres, O, <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments>,
                ) -> ::sqlx::query::QueryScalar<'q, ::sqlx::Postgres, O, <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments> {
                    e
                    #(#bind_list_for_query_scalar)*
                }
            }
        }
    } else {
        abort! { x1,
            "enum does not support"
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn compile_fail() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/fail/*.rs");
    }
}
