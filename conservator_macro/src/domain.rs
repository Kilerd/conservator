use darling::{FromDeriveInput, FromField};
use itertools::Itertools;
use proc_macro2::Span;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse2, DeriveInput};

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

fn delete_by_pk(table_name: &str, primary_field_name: &str) -> String {
    format!(
        "delete from {} where \"{}\" = $1",
        table_name, primary_field_name
    )
}
fn update_sql(table_name: &str, primary_field_name: &str, non_pk_fields: &[syn::Ident]) -> String {
    let set_part = non_pk_fields
        .iter()
        .enumerate()
        .map(|(idx, field)| format!("\"{}\" = ${}", field.to_string(), idx + 1))
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
    let x1 = parse2::<DeriveInput>(input).unwrap();
    let crud_opts: DomainOpts = DomainOpts::from_derive_input(&x1).unwrap();

    let fields = crud_opts.data.take_struct().unwrap();
    
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

    let pk_field = match pk_count.len() {
        0 => {
            return Err((
                x1.span(),
                "missing primary key, using #[domain(primary_key)] to identify",
            ));
        }
        1 => pk_count.pop().unwrap(),
        _ => {
            return Err((x1.span(), "mutliple primary key detect"));
        }
    };
    let pk_field_ident = pk_field.ident.unwrap();
    let pk_field_name = pk_field_ident.clone().to_string();
    let pk_field_type = pk_field.ty;

    let table_name = &crud_opts.table;
    let ident = crud_opts.ident.clone();
    
    // 生成 Columns 结构体名称
    let columns_struct_ident = syn::Ident::new(
        &format!("{}Columns", ident),
        ident.span(),
    );
    
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

    let delete_by_pk = delete_by_pk(&crud_opts.table, &pk_field_name);
    let update_sql = update_sql(&crud_opts.table, &pk_field_name, &non_pk_field_names);

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
    
        #[::async_trait::async_trait]
        impl ::conservator::Domain for #ident {
                const PK_FIELD_NAME: &'static str = #pk_field_name;
                const TABLE_NAME: &'static str = #table_name;
                const COLUMN_NAMES: &'static [&'static str] = &[#(#column_names),*];
    
                type PrimaryKey = #pk_field_type;

                async fn create<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>, C: ::conservator::Creatable>(
                    data: C, executor: E
                ) -> Result<Self, ::sqlx::Error> {
                    let sql = format!("INSERT INTO {} {} VALUES {} returning *", #table_name, data.get_columns(), data.get_insert_sql());
                    let mut ex = sqlx::query_as(&sql);
                    data.build_for_query_as(ex)
                        .fetch_one(executor)
                        .await
                }
                async fn batch_create<'data, 'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>, C: ::conservator::Creatable>(
                    data: Vec<C>,
                    executor: E,
                ) -> Result<(), ::sqlx::Error> {
                    if data.is_empty() {
                        return Ok(());
                    }
                    let columns = data[0].get_columns();
                    let mut insert_sql = String::new();
                    for (i, item) in data.iter().enumerate() {
                        if i > 0 {
                            insert_sql.push_str(",");
                        }
                        insert_sql.push_str(item.get_batch_insert_sql(i).as_str());
                    }
                    let sql = format!("INSERT INTO {} {} VALUES {}", #table_name, columns, insert_sql);
                    let mut ex = sqlx::query(&sql);
                    for item in data {
                        ex = item.build_for_query(ex);
                    }
                    ex.execute(executor).await?;
                    Ok(())
                }
                async fn delete_by_pk<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(pk: &Self::PrimaryKey, executor: E,) ->Result<(), ::sqlx::Error> {
                    sqlx::query(#delete_by_pk)
                    .bind(pk)
                    .execute(executor)
                    .await?;
                    Ok(())
                }
                async fn update<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(entity:Self, executor: E) ->Result<(), ::sqlx::Error> {
                    sqlx::query(#update_sql)
                        #(.bind(entity. #non_pk_field_names))*
                        .bind(entity. #pk_field_ident)
                        .execute(executor)
                        .await?;
                    Ok(())
                }
            }
    
        };
        Ok(ret)
}

#[cfg(test)]
mod test {
    use quote::quote;

    use crate::domain::handler;

    #[test]
    #[ignore]
    fn should_render() {
        let input = quote! {
            #[derive(Debug, Deserialize, Serialize, Domain, FromRow)]
            #[domain(table = "users")]
            pub struct UserEntity {
                #[domain(primary_key)]
                pub id: Uuid,
                pub username: String,
                pub email: String,
                pub password: String,
                pub role: UserRole,
                pub create_at: DateTime<Utc>,
                pub last_login_at: DateTime<Utc>,
            }
        };
        let expected_output = quote! {

            #[::async_trait::async_trait]
            impl ::conservator::Domain for UserEntity {
                const PK_FIELD_NAME: &'static str = "id";
                const TABLE_NAME: &'static str = "users";
                type PrimaryKey = Uuid;
                async fn find_by_pk<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(
                    pk: &Self::PrimaryKey,
                    executor: E
                ) -> Result<Option<Self>, ::sqlx::Error> {
                    sqlx::query_as("select * from users where \"id\" = $1")
                        .bind(pk)
                        .fetch_optional(executor)
                        .await
                }
                async fn fetch_one_by_pk<
                    'e,
                    'c: 'e,
                    E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(
                    pk: &Self::PrimaryKey,
                    executor: E
                ) -> Result<Self, ::sqlx::Error> {
                    sqlx::query_as("select * from users where \"id\" = $1")
                        .bind(pk)
                        .fetch_one(executor)
                        .await
                }
                async fn fetch_all<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(
                    executor: E
                ) -> Result<Vec<Self>, ::sqlx::Error> {
                    sqlx::query_as("select * from users")
                        .fetch_all(executor)
                        .await
                }
                async fn create<
                    'e,
                    'c: 'e,
                    E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>,
                    C: ::conservator::Creatable
                >(
                    data: C,
                    executor: E
                ) -> Result<Self, ::sqlx::Error> {
                    let sql = format!(
                        "INSERT INTO {} {} VALUES {} returning *",
                        "users",
                        data.get_columns(),
                        data.get_insert_sql()
                    );
                    let mut ex = sqlx::query_as(&sql);
                    data.build(ex).fetch_one(executor).await
                }
                async fn batch_create<'data, 'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>, C: ::conservator::Creatable>(
                    data: &'data [C],
                    executor: E,
                ) -> Result<(), ::sqlx::Error> {
                    if data.is_empty() {
                        return Ok(());
                    }
                    let columns = data[0].get_columns();
                    let insert_sql = data.iter().map(|it| it.get_insert_sql()).join(",");
                    let sql = format!("INSERT INTO {} {} VALUES {}", "users", columns, insert_sql);
                    let mut ex = sqlx::query(&sql);
                    for item in data {
                        ex = item.build(ex);
                    }
                    ex.execute(executor).await?;
                    Ok(())
                }

                async fn delete_by_pk<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(pk: &Self::PrimaryKey, executor: E,) ->Result<(), ::sqlx::Error> {
                    sqlx::query("delete from users where \"id\" = $1")
                    .bind(pk)
                .execute(executor)
                .await?;
                    Ok(())
                }

                async fn update<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(entity:Self, executor: E) ->Result<(), ::sqlx::Error> {
                    sqlx::query("UPDATE users SET \"username\" = $1, \"email\" = $2, \"password\" = $3, \"role\" = $4, \"create_at\" = $5, \"last_login_at\" = $6 WHERE \"id\" = $7")
                        .bind(entity.username)
                        .bind(entity.email)
                        .bind(entity.password)
                        .bind(entity.role)
                        .bind(entity.create_at)
                        .bind(entity.last_login_at)
                        .bind(entity.id)
                        .execute(executor)
                        .await?;
                    Ok(())
                }
            }
        };

        let stream = handler(input).unwrap();
        assert_eq!(expected_output.to_string(), stream.to_string());
    }
}
