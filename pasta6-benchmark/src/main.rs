use std::{
    cell::UnsafeCell,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use ::hyper::Uri;
use local_sync::semaphore::Semaphore;
use monoio::{time, utils, RuntimeBuilder};

mod hyper;

// 1s/10 = 100ms
const COUNT_GRAIN_PER_SEC: u32 = 10;

struct Config {
    conns_per_core: usize,
    qps_per_core: Option<usize>,
    target: Uri,
}

fn main() {
    const CONNS_PER_CORE: usize = 80;
    const CORES: &[usize] = &[3, 4, 5];
    const QPS_PER_CORE: Option<usize> = None;
    const TARGET: &str = "http://127.0.0.1:23300/todo";

    let config = Arc::new(Config {
        conns_per_core: CONNS_PER_CORE,
        qps_per_core: QPS_PER_CORE,
        target: TARGET.parse().expect("error parsing target URI"),
    });

    println!(
        "Running client.
Connection count per core: {}; Global connection count: {}
QPS limit per core: {}; Global QPS limit: {}
Target: {}
CPU slot: {:?}",
        config.conns_per_core,
        config.conns_per_core * CORES.len(),
        config.qps_per_core.unwrap_or(0),
        config.qps_per_core.unwrap_or(0) * CORES.len(),
        TARGET,
        CORES,
    );
    assert!(
        config.qps_per_core.unwrap_or(COUNT_GRAIN_PER_SEC as _) >= COUNT_GRAIN_PER_SEC as _,
        "QPS limit should be more than COUNT_GRAIN_PER_SEC"
    );

    // Count will be shared across threads.
    let count = Arc::new(AtomicUsize::new(0));
    let eps = Arc::new(AtomicU64::new(0));

    for cpu in CORES {
        let config_ = config.clone();
        let count_ = count.clone();
        let eps_ = eps.clone();
        thread::spawn(move || {
            utils::bind_to_cpu_set(Some(*cpu)).expect("error binding to CPU");
            let mut rt = RuntimeBuilder::new()
                .with_entries(2560)
                .enable_timer()
                .build()
                .expect("error building runtime");
            rt.block_on(run_thread(count_, eps_, config_));
        });
    }

    // Every second (not precise), we will print the status.
    let mut count_last = 0;
    let instant = Instant::now();
    loop {
        thread::sleep(Duration::from_secs(1));
        let count_now = count.load(Ordering::Relaxed);
        let eps_now = eps.load(Ordering::Relaxed);
        let eps_sec = instant.elapsed().as_secs_f32();
        println!(
            "{:.3}: NAdd: {}; NSum: {}; NAverage: {:.3}, LatencyAverage: {:.3} us",
            eps_sec,
            count_now - count_last,
            count_now,
            count_now as f32 / eps_sec,
            eps_now as f32 / count_now as f32,
        );
        count_last = count_now;
    }
}

// Start new tasks for each connection on the same thread.
async fn run_thread(count: Arc<AtomicUsize>, eps: Arc<AtomicU64>, config: Arc<Config>) {
    let mut hdrs = Vec::with_capacity(config.conns_per_core);

    // count_tls and sem will be shared across tasks.
    let count_tls = Rc::new(UnsafeCell::new(0));
    let eps_tls = Rc::new(UnsafeCell::new(0));
    let grain_n = config.qps_per_core.unwrap_or(0) / COUNT_GRAIN_PER_SEC as usize;
    let sem = config
        .qps_per_core
        .map(|_| Rc::new(Semaphore::new(grain_n)));

    for _ in 0..config.conns_per_core {
        hdrs.push(monoio::spawn(run_conn(
            count_tls.clone(),
            eps_tls.clone(),
            sem.clone(),
            config.target.clone(),
        )));
    }
    let mut interval = time::interval(Duration::from_secs(1) / COUNT_GRAIN_PER_SEC);
    loop {
        interval.tick().await;
        let c = unsafe { &mut *count_tls.get() };
        let e = unsafe { &mut *eps_tls.get() };
        count.fetch_add(*c, Ordering::Relaxed);
        *c = 0;
        eps.fetch_add(*e, Ordering::Relaxed);
        *e = 0;
        if let Some(s) = sem.as_ref() {
            s.add_permits(grain_n);
        }
    }
}

async fn run_conn(
    count: Rc<UnsafeCell<usize>>,
    eps: Rc<UnsafeCell<u64>>,
    qps_per_conn: Option<Rc<Semaphore>>,
    target: Uri,
) {
    let client = hyper::build();

    loop {
        if let Some(s) = qps_per_conn.as_ref() {
            s.acquire().await.unwrap().forget();
        }

        let begin = Instant::now();
        hyper::get(&client, target.clone())
            .await
            .expect("connection exit");
        let eps_ = begin.elapsed().as_micros() as u64;
        unsafe {
            *count.get() += 1;
            *eps.get() += eps_;
        }
    }
}
