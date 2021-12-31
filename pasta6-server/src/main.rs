use hyper::Body;
use hyper::Request;
use monoio::utils;
use monoio::RuntimeBuilder;

use std::thread;

fn main() {
    const CORES: &[usize] = &[0, 1, 2];
    const ADDR: &str = "0.0.0.0:23300";

    println!(
        "running HTTP server
address: {}
cores: {:?}",
        ADDR, CORES
    );

    let db = sled::open("database").expect("error opening pasta6.db");

    let mut threads = Vec::new();
    for cpu in CORES {
        let db = db.clone();
        let handle = thread::spawn(move || {
            utils::bind_to_cpu_set(Some(*cpu)).expect("error binding to CPU");
            let mut rt = RuntimeBuilder::new()
                .with_entries(32768)
                .build()
                .expect("error building monoio runtime");
            let handler = move |req: Request<Body>| {
                let db = db.clone();
                async move { pasta6::handler(&db, req).await }
            };
            rt.block_on(pasta6::serve(ADDR, handler))
                .expect("error serving HTTP requests");
        });
        threads.push(handle);
    }

    for handle in threads {
        handle.join().expect("error joining handle");
    }
}
