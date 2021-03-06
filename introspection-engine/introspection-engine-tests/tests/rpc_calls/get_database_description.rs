use barrel::types;
use introspection_engine_tests::{assert_eq_schema, test_api::*, BarrelMigrationExecutor};
use test_macros::test_each_connector;

async fn setup_blog(barrel: &BarrelMigrationExecutor) -> crate::TestResult {
    barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
                t.add_column("string", types::text());
            });
        })
        .await?;

    Ok(())
}

#[test_each_connector(tags("mysql_5_6", "mariadb"))]
async fn database_description_for_mysql_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = r#"{"tables":[{"name":"Blog","columns":[{"name":"id","tpe":{"full_data_type":"int(11)","family":"Int","arity":"Required","native_type":"Int"},"default":null,"auto_increment":true},{"name":"string","tpe":{"full_data_type":"text","family":"String","arity":"Required","native_type":"Text"},"default":null,"auto_increment":false}],"indices":[],"primary_key":{"columns":["id"],"sequence":null,"constraint_name":null},"foreign_keys":[]}],"enums":[],"sequences":[],"views":[],"procedures":[]}"#;

    assert_eq_schema!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("mysql_8"))]
async fn database_description_for_mysql_8_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = r#"{"tables":[{"name":"Blog","columns":[{"name":"id","tpe":{"full_data_type":"int","family":"Int","arity":"Required","native_type":"Int"},"default":null,"auto_increment":true},{"name":"string","tpe":{"full_data_type":"text","family":"String","arity":"Required","native_type":"Text"},"default":null,"auto_increment":false}],"indices":[],"primary_key":{"columns":["id"],"sequence":null,"constraint_name":null},"foreign_keys":[]}],"enums":[],"sequences":[],"views":[],"procedures":[]}"#;

    assert_eq_schema!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn database_description_for_postgres_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = r#"{"tables":[{"name":"Blog","columns":[{"name":"id","tpe":{"full_data_type":"int4","family":"Int","arity":"Required","native_type":"Integer"},"default":{"kind":{"Sequence":"Blog_id_seq"},"constraint_name":null},"auto_increment":true},{"name":"string","tpe":{"full_data_type":"text","family":"String","arity":"Required","native_type":"Text"},"default":null,"auto_increment":false}],"indices":[],"primary_key":{"columns":["id"],"sequence":{"name":"Blog_id_seq"},"constraint_name":"Blog_pkey"},"foreign_keys":[]}],"enums":[],"sequences":[{"name":"Blog_id_seq"}],"views":[],"procedures":[]}"#;

    assert_eq_schema!(expected, api.get_database_description().await?);

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn database_description_for_sqlite_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    let expected = r#"{"tables":[{"name":"Blog","columns":[{"name":"id","tpe":{"full_data_type":"INTEGER","family":"Int","arity":"Required","native_type":null},"default":null,"auto_increment":true},{"name":"string","tpe":{"full_data_type":"TEXT","family":"String","arity":"Required","native_type":null},"default":null,"auto_increment":false}],"indices":[],"primary_key":{"columns":["id"],"sequence":null,"constraint_name":null},"foreign_keys":[]}],"enums":[],"sequences":[],"views":[],"procedures":[]}"#;

    assert_eq_schema!(expected, api.get_database_description().await?);

    Ok(())
}

//cant assert the string since the PK constraint name is random
//just checking it does not error
#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn database_description_for_mssql_should_work(api: &TestApi) -> crate::TestResult {
    setup_blog(&api.barrel()).await?;

    api.get_database_description().await?;

    Ok(())
}
