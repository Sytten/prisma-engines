use query_test_macros::connector_test;
use query_tests_setup::*;
use serde_json::Value;
use std::convert::TryFrom;

pub type TestResult = anyhow::Result<()>;

pub mod schemas {
    pub fn some_common_schema() -> String {
        "model A {
            #id(id, Int, @id)
            field String?
            #relation(bs, B, ...)
        }"
        .to_owned()
    }
}

// The mod name dictates the db name. If the name is `some_spec`
// then, for example, the MySQL db should be (similar to) `some_spec` as well.
#[cfg(test)]
// #[before_each(before_each_handler)] // Hook to run before each test.
// #[schema(schema_handler)] // Schema for all contained tests. Allows us to cache runners maybe.
mod some_spec {
    use query_tests_setup::Runner;

    use super::*;

    // fn before_each_handler(runner: &Runner) {
    //     // Maybe we don't need this.
    //     runner.truncate_data(); // Actually, this should always happen for a connector test.
    //     test_data(); // This can also be done in each test manually or by convention.
    // }

    // fn test_data() {
    //     todo!()
    // }

    // Handler that returns a schema template to use for rendering.
    // Template rendering can be bypassed by simply not using the template strings.
    // Common schema handlers to use should be in a central place.
    fn schema_handler() -> String {
        "model A {
            #id(id, Int, @id)
            field String?
            #relation(bs, B, ...)
        }"
        .to_owned()
    }

    // (
    //     schema(schema_handler), // Override or manual set of schema to use.
    //     only(Postgres), // Only run for certain connectors, xor with `exclude`
    //     exclude(SqlServer) // Run for all except certain connectors, xor with `only`
    //     // If none of the two above are specified all connectors are run.
    // )

    #[connector_test(schema(schema_handler), only(SqlServer(2017)))] // , Postgres("test-4", 1), MySql, Wurst
    fn ideal_api_test(runner: &Runner) {
        let result = runner.query(
            "
            mutation {
                createOneA(data: {...}) { id }
            }
        ",
        );

        assert_eq!(result.to_string(), r#"{"data":{"createOneA":[...]}}"#);
    }
}