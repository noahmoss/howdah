use tokio_postgres::{Client, NoTls};

// Each test owns a session. Temporary tables disappear when its runtime closes.
pub async fn connect() -> Client {
    let connection_string =
        std::env::var("HOWDAH_TEST_DATABASE_URL").expect("set HOWDAH_TEST_DATABASE_URL");
    let (client, connection) = tokio_postgres::connect(&connection_string, NoTls)
        .await
        .unwrap();
    tokio::spawn(async move {
        connection.await.expect("test database connection failed");
    });
    client
}
