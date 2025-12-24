use std::collections::HashSet;
use std::str::FromStr;

use itertools::Itertools;
use proc_macro2::Span;
use quote::{format_ident, quote};
use regex::Regex;
use strum::EnumString;
use syn::spanned::Spanned;
use syn::{
    AngleBracketedGenericArguments, Expr, ItemFn, Lit, PathArguments, ReturnType, Stmt, Type,
    parse2,
};

fn extract_inner_type<'a>(ty: &'a Type, wrapper: &'a str) -> Option<&'a Type> {
    if let Type::Path(syn::TypePath { qself: None, path }) = ty {
        if let Some(segment) = path.segments.last() {
            if segment.ident == wrapper {
                if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    args, ..
                }) = &segment.arguments
                {
                    if let Some(syn::GenericArgument::Type(inner_type)) = args.first() {
                        return Some(inner_type);
                    }
                }
            }
        }
    }
    None
}

#[derive(Debug, EnumString)]
#[strum(serialize_all = "snake_case")]
enum Action {
    Fetch,
    Exists,
    Find,
    FetchAll,
    Execute,
}

impl Action {
    fn build_conservator_query(
        &self,
        fields: &[String],
        fetch_model: &proc_macro2::TokenStream,
        sql: String,
    ) -> proc_macro2::TokenStream {
        let fields = fields
            .iter()
            .filter(|&field| !field.eq("executor"))
            .map(|field| format_ident!("{}", field))
            .collect_vec();

        match self {
            Action::Fetch => {
                quote! {
                    let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![#(&#fields,)*];
                    let row = executor.query_one(#sql, &params).await?;
                    #fetch_model::from_row(&row)
                }
            }
            Action::Exists => {
                let exist_wrapper_sql = format!("select exists({})", sql);
                quote! {
                    let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![#(&#fields,)*];
                    executor.query_scalar(#exist_wrapper_sql, &params).await
                }
            }
            Action::Find => {
                quote! {
                    let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![#(&#fields,)*];
                    match executor.query_opt(#sql, &params).await? {
                        Some(row) => Ok(Some(#fetch_model::from_row(&row)?)),
                        None => Ok(None),
                    }
                }
            }
            Action::FetchAll => {
                quote! {
                    let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![#(&#fields,)*];
                    let rows = executor.query(#sql, &params).await?;
                    rows.iter().map(|row| #fetch_model::from_row(row)).collect()
                }
            }
            Action::Execute => {
                quote! {
                    let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![#(&#fields,)*];
                    executor.execute(#sql, &params).await?;
                    Ok(())
                }
            }
        }
    }

    fn extract_and_build_ret_type(
        &self,
        ident: &ReturnType,
    ) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), (Span, &'static str)> {
        let span = ident.span();
        match ident {
            ReturnType::Default => Err((span, "default return type does not support")),
            ReturnType::Type(_, inner) => match self {
                Action::Fetch => Ok((quote! {#inner}, quote! { #inner })),
                Action::Exists => Ok((quote! {bool}, quote! { bool })),
                Action::Find => {
                    let Some(inner_type) = extract_inner_type(inner, "Option") else {
                        return Err((span, "find method need a option type"));
                    };
                    Ok((quote! {#inner_type}, quote! { #inner }))
                }
                Action::FetchAll => {
                    let Some(inner_type) = extract_inner_type(inner, "Vec") else {
                        return Err((span, "fetchall method need a vec type"));
                    };
                    Ok((quote! {#inner_type}, quote! { #inner }))
                }
                Action::Execute => Ok((quote! {()}, quote! { () })),
            },
        }
    }
}

pub(crate) fn handler(
    args: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream, (Span, &'static str)> {
    let arg = args.to_string();
    let action = match Action::from_str(&arg) {
        Ok(action) => action,
        Err(_) => return Err((args.span(), "unknown action type")),
    };

    let input_span = input.span();
    let method = match parse2::<ItemFn>(input) {
        Ok(func) => func,
        Err(_) => return Err((input_span, "unknown action type")),
    };

    let vis = &method.vis;
    let ident = &method.sig.ident;
    let inputs = &method.sig.inputs;

    let output = &method.sig.output;

    let (fetch_model, return_type) = action.extract_and_build_ret_type(output)?;
    let body = &method.block;
    let body: Vec<proc_macro2::TokenStream> = body
        .stmts
        .iter()
        .map(|stmt| match &stmt {
            Stmt::Expr(Expr::Lit(expr_lit)) => match &expr_lit.lit {
                Lit::Str(lit_str) => {
                    let mut sql = lit_str.value();
                    let re = Regex::new(r"[^:]:(\w+)").unwrap();
                    let matched: HashSet<String> = re
                        .captures_iter(&sql)
                        .map(|mat| mat[1].to_string())
                        .collect();
                    let matched_fields = matched.into_iter().collect_vec();

                    matched_fields.iter().enumerate().for_each(|(idx, field)| {
                        sql = sql.replace(&format!(":{}", field), &format!("${}", idx + 1));
                    });
                    let query_stmt =
                        action.build_conservator_query(&matched_fields[..], &fetch_model, sql);
                    quote!( #query_stmt)
                }
                _ => {
                    quote!( #stmt )
                }
            },
            _ => quote!( #stmt ),
        })
        .collect();

    let inputs = if inputs.is_empty() {
        quote! {}
    } else if inputs.trailing_punct() {
        quote! { #inputs}
    } else {
        quote! { #inputs,}
    };
    let ret = quote! {
        #vis async fn #ident<E: ::conservator::Executor>(#inputs executor: &E) -> Result<#return_type, ::conservator::Error> {
            #(#body )*
        }
    };
    Ok(ret)
}

#[cfg(test)]
mod test {
    use crate::sql::handler;

    #[test]
    fn should_generate_fetch_sql_function() {
        use quote::quote;
        let args = quote! { find };
        let input = quote! {
            pub async fn find_user(email: &str) -> Option<UserEntity> {
                "select * from users where email = :email"
            }
        };

        let expected = quote! {
            pub async fn find_user<E: ::conservator::Executor>(
                email: &str,
                executor: &E
            ) -> Result<Option<UserEntity>, ::conservator::Error> {
                let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![&email,];
                match executor.query_opt("select * from users where email = $1", &params).await? {
                    Some(row) => Ok(Some(UserEntity::from_row(&row)?)),
                    None => Ok(None),
                }
            }
        };
        assert_eq!(
            expected.to_string(),
            handler(args, input).unwrap().to_string()
        );
    }

    #[test]
    fn should_generate_for_linked_domain() {
        use quote::quote;
        let args = quote! { find };
        let input = quote! {
            pub async fn find_user(&self) -> Option<UserEntity> {
                let id = self.id;
                "select * from users where email = :id"
            }
        };

        let expected = quote! {
            pub async fn find_user<E: ::conservator::Executor>(
                &self,
                executor: &E
            ) -> Result<Option<UserEntity>, ::conservator::Error> {
                let id = self.id;
                let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![&id,];
                match executor.query_opt("select * from users where email = $1", &params).await? {
                    Some(row) => Ok(Some(UserEntity::from_row(&row)?)),
                    None => Ok(None),
                }
            }
        };
        assert_eq!(
            expected.to_string(),
            handler(args, input).unwrap().to_string()
        );
    }

    #[test]
    fn args_with_tailing_comma() {
        use quote::quote;
        let args = quote! { find };
        let input = quote! {
            pub async fn find_user(id: i32, ) -> Option<UserEntity> {
                "select * from users where email = :id"
            }
        };

        let expected = quote! {
            pub async fn find_user<E: ::conservator::Executor>(
                id: i32,
                executor: &E
            ) -> Result<Option<UserEntity>, ::conservator::Error> {
                let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![&id,];
                match executor.query_opt("select * from users where email = $1", &params).await? {
                    Some(row) => Ok(Some(UserEntity::from_row(&row)?)),
                    None => Ok(None),
                }
            }
        };
        assert_eq!(
            expected.to_string(),
            handler(args, input).unwrap().to_string()
        );
    }

    #[test]
    fn args_without_tailing_comma() {
        use quote::quote;
        let args = quote! { find };
        let input = quote! {
            pub async fn find_user(id: i32 ) -> Option<UserEntity> {
                "select * from users where email = :id"
            }
        };

        let expected = quote! {
            pub async fn find_user<E: ::conservator::Executor>(
                id: i32,
                executor: &E
            ) -> Result<Option<UserEntity>, ::conservator::Error> {
                let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![&id,];
                match executor.query_opt("select * from users where email = $1", &params).await? {
                    Some(row) => Ok(Some(UserEntity::from_row(&row)?)),
                    None => Ok(None),
                }
            }
        };
        assert_eq!(
            expected.to_string(),
            handler(args, input).unwrap().to_string()
        );
    }

    #[test]
    fn should_work_with_pg_double_mark() {
        use quote::quote;
        let args = quote! { find };
        let input = quote! {
            pub async fn find_user() -> Option<UserEntity> {
                "select * from users where datetime + '14 days'::interval > now()"
            }
        };

        let expected = quote! {
            pub async fn find_user<E: ::conservator::Executor>(
                executor: &E
            ) -> Result<Option<UserEntity>, ::conservator::Error> {
                let params: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = vec![];
                match executor.query_opt("select * from users where datetime + '14 days'::interval > now()", &params).await? {
                    Some(row) => Ok(Some(UserEntity::from_row(&row)?)),
                    None => Ok(None),
                }
            }
        };
        assert_eq!(
            expected.to_string(),
            handler(args, input).unwrap().to_string()
        );
    }
}
