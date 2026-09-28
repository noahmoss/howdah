use howdah_core::run_sql;
use tokio_postgres::{Error, NoTls};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (client, connection) =
        tokio_postgres::connect("host=localhost user=noahmoss dbname=howdah_dev", NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let results = run_sql(&client, "SELECT * FROM bird").await;
    println!("{:?}", results);

    Ok(())
}
