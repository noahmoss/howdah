use howdah_core::run_query;
use tokio_postgres::{Error, NoTls};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (client, connection) =
        tokio_postgres::connect("host=localhost user=noahmoss dbname=bird-flocks", NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let results = run_query(&client, "SELECT * FROM bird").await;
    println!("{:?}", results);

    Ok(())
}
