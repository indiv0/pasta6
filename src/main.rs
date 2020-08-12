use warp::Filter;

async fn create_db_connection() -> Result<tokio_postgres::Client, tokio_postgres::Error>{
    // Connect to the database.
    let (client, conn) = tokio_postgres::connect("host=localhost user=postgres password=password", tokio_postgres::NoTls).await?;

    // The connection object performs the communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}

#[tokio::main]
async fn main() -> Result<(), tokio_postgres::Error> {
    let _client = create_db_connection().await?;

    let health = warp::path!("health")
        .map(|| warp::http::StatusCode::OK);

    let paste = warp::post()
        .and(warp::path("paste"))
        // Only accept bodies smaller than 16kb
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::bytes())
        .map(|bytes| {
            format!("bytes = {:?}", bytes)
        });

    let routes = health
        .or(paste);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;

    Ok(())
}
