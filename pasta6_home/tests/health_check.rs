use pasta6_core::{create_db_pool, init_tracing, ServerConfig};
use pasta6_home::run;
use std::net::TcpListener;

const SITE: &str = "home";

struct TestApp {
    address: String,
}

fn spawn_app() -> TestApp {
    let config = ServerConfig::new();
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let pool = create_db_pool(SITE).expect("create db pool error");

    let server = run(config, listener, pool);
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
