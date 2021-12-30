use monoio::utils;
use monoio::RuntimeBuilder;

use std::thread;

fn main() {
    const CORES: usize = 4;
    const ADDR: &str = "0.0.0.0:23300";

    println!("running HTTP server on {} with {} cores", ADDR, CORES);

    let mut threads = Vec::new();
    for cpu in 0..CORES {
        let handle = thread::spawn(move || {
            utils::bind_to_cpu_set(Some(cpu)).expect("error binding to CPU");
            let mut rt = RuntimeBuilder::new()
                .with_entries(32768)
                .build()
                .expect("error building monoio runtime");
            rt.block_on(pasta6::serve(ADDR, pasta6::handler))
                .expect("error serving HTTP requests");
        });
        threads.push(handle);
    }

    for handle in threads {
        handle.join().expect("error joining handle");
    }
}
