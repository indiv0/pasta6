use pasta6_core::{init_tracing, create_db_pool};
use pasta6_paste::run;
use std::net::TcpListener;

struct TestApp {
    address: String,
}

fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let pool = create_db_pool().expect("create db pool error");

    let server = run(listener, pool);
    tokio::spawn(server);

    TestApp { address }
}

#[tokio::test]
async fn health_check_returns_200() {
    better_panic::install();
    init_tracing("pasta6_meta");

    let app = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health", app.address))
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(200, response.status().as_u16());
}
