use darling::{FromDeriveInput, FromField};
use itertools::Itertools;
use proc_macro2::Span;
use quote::quote;
use syn::spanned::Spanned;
use syn::{DeriveInput, parse2};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(domain))]
struct DomainOpts {
    ident: syn::Ident,
    table: String,
    data: darling::ast::Data<darling::util::Ignored, DomainFieldOpt>,
}

#[derive(Debug, FromField)]
#[darling(attributes(domain))]
struct DomainFieldOpt {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(default)]
    primary_key: Option<bool>,
}

fn update_sql(table_name: &str, primary_field_name: &str, non_pk_fields: &[syn::Ident]) -> String {
    let set_part = non_pk_fields
        .iter()
        .enumerate()
        .map(|(idx, field)| format!("\"{}\" = ${}", field, idx + 1))
        .join(", ");
    format!(
        "UPDATE {} SET {} WHERE \"{}\" = ${}",
        table_name,
        set_part,
        primary_field_name,
        non_pk_fields.len() + 1
    )
}

pub(crate) fn handler(
    input: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream, (Span, &'static str)> {
    let x1 = parse2::<DeriveInput>(input.clone())
        .map_err(|e| (e.span(), "failed to parse struct definition"))?;

    let crud_opts: DomainOpts = DomainOpts::from_derive_input(&x1)
        .map_err(|_| (x1.span(), "failed to parse #[domain] attributes"))?;

    let fields = crud_opts.data.take_struct().ok_or((
        x1.span(),
        "Domain can only be derived for structs, not enums",
    ))?;

    // 收集所有字段信息用于生成 Columns 结构体
    let all_fields: Vec<_> = fields
        .fields
        .iter()
        .filter_map(|field| {
            field.ident.clone().map(|ident| {
                let is_pk = field.primary_key == Some(true);
                (ident, field.ty.clone(), is_pk)
            })
        })
        .collect();

    let non_pk_field_names = fields
        .fields
        .iter()
        .filter(|field| field.primary_key.is_none())
        .filter_map(|field| field.ident.clone())
        .collect_vec();

    let mut pk_count = fields
        .fields
        .into_iter()
        .filter(|field| field.primary_key == Some(true))
        .collect_vec();

    // Validate primary key configuration
    let pk_field = match pk_count.len() {
        0 => {
            return Err((
                x1.span(),
                "missing primary key, using #[domain(primary_key)] to identify",
            ));
        }
        1 => pk_count
            .pop()
            .expect("BUG: pk_count.len() == 1 but pop() returned None"),
        _ => {
            return Err((x1.span(), "mutliple primary key detect"));
        }
    };

    let pk_field_ident = pk_field.ident.ok_or((
        x1.span(),
        "primary key field must have a name (tuple structs not supported)",
    ))?;
    let pk_field_name = pk_field_ident.to_string();
    let pk_field_type = pk_field.ty;

    let table_name = &crud_opts.table;
    let ident = crud_opts.ident.clone();

    // 生成 Columns 结构体名称
    let columns_struct_ident = syn::Ident::new(&format!("{}Columns", ident), ident.span());

    // 生成 Columns 结构体的字段定义
    let columns_fields = all_fields.iter().map(|(field_ident, field_ty, _)| {
        quote! {
            pub #field_ident: ::conservator::Field<#field_ty>
        }
    });

    // 生成 COLUMNS 常量的字段初始化
    let columns_init = all_fields.iter().map(|(field_ident, _, is_pk)| {
        let field_name = field_ident.to_string();
        quote! {
            #field_ident: ::conservator::Field::new(#field_name, #table_name, #is_pk)
        }
    });

    // 生成列名数组
    let column_names: Vec<String> = all_fields
        .iter()
        .map(|(ident, _, _)| ident.to_string())
        .collect();

    let update_sql = update_sql(&crud_opts.table, &pk_field_name, &non_pk_field_names);

    // 生成 FromRow 的字段名
    let field_idents: Vec<_> = all_fields
        .iter()
        .map(|(ident, _, _)| ident.clone())
        .collect();
    let field_names_str: Vec<String> = all_fields
        .iter()
        .map(|(ident, _, _)| ident.to_string())
        .collect();

    let ret = quote! {
    /// 包含 #ident 所有字段元信息的结构体
    #[derive(Debug, Clone, Copy)]
    pub struct #columns_struct_ident {
        #(#columns_fields),*
    }

    impl #ident {
        /// 所有字段的元信息
        pub const COLUMNS: #columns_struct_ident = #columns_struct_ident {
            #(#columns_init),*
        };
    }

    // 实现 Selectable trait
    impl ::conservator::Selectable for #ident {
        const COLUMN_NAMES: &'static [&'static str] = &[#(#column_names),*];

        fn from_row(row: &::tokio_postgres::Row) -> Result<Self, ::conservator::Error> {
            use ::conservator::SqlTypeWrapper;
            Ok(Self {
                #(#field_idents: { let wrapper: SqlTypeWrapper<_> = row.try_get(#field_names_str)?; wrapper.0 }),*
            })
        }
    }


    #[::async_trait::async_trait]
    impl ::conservator::Domain for #ident {
        const PK_FIELD_NAME: &'static str = #pk_field_name;
        const TABLE_NAME: &'static str = #table_name;

        type PrimaryKey = #pk_field_type;

        async fn save<E: ::conservator::Executor>(
            &self,
            executor: &E,
        ) -> Result<(), ::conservator::Error> {
            use ::conservator::{IntoValue, Value};

            // 收集所有参数值
            let mut values: Vec<Value> = vec![
                #(::conservator::IntoValue::into_value(self.#non_pk_field_names.clone())),*
            ];
            values.push(::conservator::IntoValue::into_value(self.#pk_field_ident.clone()));

            // 将 Value 转换为 ToSql 参数
            let params: Vec<Box<dyn ::tokio_postgres::types::ToSql + Sync + Send + 'static>> = values
                .into_iter()
                .map(|v| v.to_tokio_sql_param())
                .collect::<Result<Vec<_>, _>>()?;

            // 转换为引用数组
            let param_refs: Vec<&(dyn ::tokio_postgres::types::ToSql + Sync)> = params
                .iter()
                .map(|p| p.as_ref() as &(dyn ::tokio_postgres::types::ToSql + Sync))
                .collect();

            executor.execute(#update_sql, &param_refs).await?;
            Ok(())
        }
    }

    };
    Ok(ret)
}
