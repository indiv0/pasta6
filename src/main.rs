use warp::Filter;

#[tokio::main]
async fn main() {
    let paste = warp::post()
        .and(warp::path("paste"))
        // Only accept bodies smaller than 16kb
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::bytes())
        .map(|bytes| {
            format!("bytes = {:?}", bytes)
        });

    warp::serve(paste)
        .run(([127, 0, 0, 1], 3030))
        .await;
}
