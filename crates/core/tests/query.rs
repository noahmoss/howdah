mod common;

use howdah_core::run_query;

#[tokio::test]
#[ignore = "requires PostgreSQL; set HOWDAH_TEST_DATABASE_URL and run with --ignored"]
async fn preserves_commands_without_row_counts() {
    let client = common::connect().await;
    let results = run_query(&client, "CREATE TEMP TABLE command_tags (id INT)")
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let result = results[0].as_ref().unwrap();
    assert_eq!(result.tag, "CREATE TABLE");
    assert_eq!(result.row_count, None);
    assert_eq!(result.cols, None);
    assert!(result.rows.is_empty());
}

#[tokio::test]
#[ignore = "requires PostgreSQL; set HOWDAH_TEST_DATABASE_URL and run with --ignored"]
async fn preserves_zero_rows_affected() {
    let client = common::connect().await;
    client
        .batch_execute("CREATE TEMP TABLE command_tags (id INT)")
        .await
        .unwrap();
    let results = run_query(&client, "UPDATE command_tags SET id = 3")
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let result = results[0].as_ref().unwrap();
    assert_eq!(result.tag, "UPDATE 0");
    assert_eq!(result.row_count, Some(0));
    assert_eq!(result.cols, None);
    assert!(result.rows.is_empty());
}

#[tokio::test]
#[ignore = "requires PostgreSQL; set HOWDAH_TEST_DATABASE_URL and run with --ignored"]
async fn preserves_nonzero_rows_affected() {
    let client = common::connect().await;
    client
        .batch_execute(
            "CREATE TEMP TABLE command_tags (id INT);
             INSERT INTO command_tags VALUES (1), (2);",
        )
        .await
        .unwrap();
    let results = run_query(&client, "DELETE FROM command_tags")
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let result = results[0].as_ref().unwrap();
    assert_eq!(result.tag, "DELETE 2");
    assert_eq!(result.row_count, Some(2));
    assert_eq!(result.cols, None);
    assert!(result.rows.is_empty());
}

#[tokio::test]
#[ignore = "requires PostgreSQL; set HOWDAH_TEST_DATABASE_URL and run with --ignored"]
async fn preserves_insert_tag_with_returning_rows() {
    let client = common::connect().await;
    client
        .batch_execute("CREATE TEMP TABLE command_tags (id INT)")
        .await
        .unwrap();
    let results = run_query(
        &client,
        "INSERT INTO command_tags VALUES (1), (2) RETURNING id",
    )
    .await
    .unwrap();

    assert_eq!(results.len(), 1);
    let result = results[0].as_ref().unwrap();
    assert_eq!(result.tag, "INSERT 0 2");
    assert_eq!(result.row_count, Some(2));
    assert_eq!(result.cols, Some(vec!["id".into()]));
    assert_eq!(result.rows, vec![vec!["1"], vec!["2"]]);
}

#[tokio::test]
#[ignore = "requires PostgreSQL; set HOWDAH_TEST_DATABASE_URL and run with --ignored"]
async fn preserves_columns_for_empty_result_sets() {
    let client = common::connect().await;
    let results = run_query(&client, "SELECT 1 AS id WHERE false")
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let result = results[0].as_ref().unwrap();
    assert_eq!(result.tag, "SELECT 0");
    assert_eq!(result.row_count, Some(0));
    assert_eq!(result.cols, Some(vec!["id".into()]));
    assert!(result.rows.is_empty());
}

#[tokio::test]
#[ignore = "requires PostgreSQL; set HOWDAH_TEST_DATABASE_URL and run with --ignored"]
async fn keeps_statement_results_separate() {
    let client = common::connect().await;
    let results = run_query(
        &client,
        "SELECT 1 AS first;
         SELECT 2 AS second;
         CREATE TEMP TABLE command_tags (id INT);",
    )
    .await
    .unwrap();

    assert_eq!(results.len(), 3);
    let first = results[0].as_ref().unwrap();
    assert_eq!(first.tag, "SELECT 1");
    assert_eq!(first.row_count, Some(1));
    assert_eq!(first.cols, Some(vec!["first".into()]));
    assert_eq!(first.rows, vec![vec!["1"]]);

    let second = results[1].as_ref().unwrap();
    assert_eq!(second.tag, "SELECT 1");
    assert_eq!(second.row_count, Some(1));
    assert_eq!(second.cols, Some(vec!["second".into()]));
    assert_eq!(second.rows, vec![vec!["2"]]);

    let command = results[2].as_ref().unwrap();
    assert_eq!(command.tag, "CREATE TABLE");
    assert_eq!(command.row_count, None);
    assert_eq!(command.cols, None);
    assert!(command.rows.is_empty());
}

#[tokio::test]
#[ignore = "requires PostgreSQL; set HOWDAH_TEST_DATABASE_URL and run with --ignored"]
async fn skips_queries_without_statements() {
    let client = common::connect().await;
    for sql in ["", "  ", "-- comment", "/* comment */", "; ;"] {
        assert!(run_query(&client, sql).await.unwrap().is_empty(), "{sql:?}");
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; set HOWDAH_TEST_DATABASE_URL and run with --ignored"]
async fn skips_empty_statements_around_a_query() {
    let client = common::connect().await;
    let results = run_query(&client, "; SELECT 1 WHERE false; ; -- comment")
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let result = results[0].as_ref().unwrap();
    assert_eq!(result.tag, "SELECT 0");
    assert_eq!(result.row_count, Some(0));
    assert!(result.cols.is_some());
    assert!(result.rows.is_empty());
}
