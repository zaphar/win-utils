use anyhow;
use gflags;
use log::{debug, error, info};
use nursery;
use nursery::thread;
use nursery::Waitable;
use prometheus;
use prometheus::Encoder;
use winapi_perf_wrapper::{ValueStream, PDH};

gflags::define!(
    /// Print this help text.
    -h,
    --help = false
);

gflags::define! {
    /// Delay between collections from windows performance counters.
    --delaySecs: u64 = 10
}

gflags::define! {
    /// address:port to listen on for exporting variables prometheus style.
    --listenHost = "0.0.0.0:8080"
}

gflags::define! {
    /// Enable debug logging
    --debug = false
}

fn usage(code: i32) {
    println!("win-prom-node-exporter <flags>");
    println!("");
    gflags::print_help_and_exit(code);
}

fn main() -> anyhow::Result<()> {
    gflags::parse();
    if HELP.flag {
        usage(0);
    }

    let level = if DEBUG.flag { 2 } else { 3 };
    stderrlog::new()
        .verbosity(level)
        .timestamp(stderrlog::Timestamp::Millisecond)
        .init()?;

    let mut parent = nursery::Nursery::new();

    let prom_cpu_pct_gauge = prometheus::GaugeVec::new(
        prometheus::Opts::new(
            "cpu_total_pct",
            r"\Processor Information(_Total)\% Processor Time",
        ),
        &[],
    )?;
    let prom_mem_available_guage = prometheus::GaugeVec::new(
        prometheus::Opts::new("mem_available_bytes", r"\Memory\Available Bytes"),
        &[],
    )?;
    let prom_mem_cache_guage = prometheus::GaugeVec::new(
        prometheus::Opts::new("mem_cache_bytes", r"\Memory\Cache Bytes"),
        &[],
    )?;
    let prom_mem_committed_guage = prometheus::GaugeVec::new(
        prometheus::Opts::new("mem_committed_bytes", r"\Memory\Committed Bytes"),
        &[],
    )?;
    debug!("Setting up registry of prometheus metrics");
    let registry = prometheus::Registry::new();
    registry.register(Box::new(prom_cpu_pct_gauge.clone()))?;
    registry.register(Box::new(prom_mem_available_guage.clone()))?;
    registry.register(Box::new(prom_mem_cache_guage.clone()))?;
    registry.register(Box::new(prom_mem_committed_guage.clone()))?;
    let render_thread = thread::Handle::new(move || {
        info!("Starting server on {}", LISTENHOST.flag);
        let server = tiny_http::Server::http(LISTENHOST.flag).unwrap();
        loop {
            info!("Waiting for request");
            match server.recv() {
                Ok(req) => {
                    info!("Handling request");
                    let mut buffer = vec![];
                    // Gather the metrics.
                    let encoder = prometheus::TextEncoder::new();
                    let metric_families = registry.gather();
                    encoder.encode(&metric_families, &mut buffer).unwrap();

                    let response = tiny_http::Response::from_data(buffer).with_status_code(200);
                    if let Err(e) = req.respond(response) {
                        error!("Error responding to request {}", e);
                    }
                }
                Err(e) => {
                    error!("Invalid http request! {}", e);
                }
            }
        }
    });

    let collection_thread = thread::Handle::new(move || {
        debug!("Opening PDH Performance counter query");
        let mut pdh = PDH::new();
        let query = pdh.open_query().unwrap();
        debug!("Adding counters to query");
        let cpu_stream = query
            .get_value_stream_from_path::<_, i32>(
                r"\Processor Information(_Total)\% Processor Time",
            )
            .unwrap();
        let mem_available_stream = query
            .get_value_stream_from_path::<_, i64>(r"\Memory\Available Bytes")
            .unwrap();
        let mem_cache_stream = query
            .get_value_stream_from_path::<_, i64>(r"\Memory\Cache Bytes")
            .unwrap();
        let mem_committed_stream = query
            .get_value_stream_from_path::<_, i64>(r"\Memory\Committed Bytes")
            .unwrap();
        info!("Starting collection thread");
        loop {
            if let Ok(v) = cpu_stream.next() {
                prom_cpu_pct_gauge
                    .with(&prometheus::labels! {})
                    .set(v as f64);
            }
            if let Ok(v) = mem_available_stream.next() {
                prom_mem_available_guage
                    .with(&prometheus::labels! {})
                    .set(v as f64);
            }
            if let Ok(v) = mem_cache_stream.next() {
                prom_mem_cache_guage
                    .with(&prometheus::labels! {})
                    .set(v as f64);
            }
            if let Ok(v) = mem_committed_stream.next() {
                prom_mem_committed_guage
                    .with(&prometheus::labels! {})
                    .set(v as f64);
            }
            debug!("Sleeping until next collection");
            std::thread::sleep(std::time::Duration::from_secs(DELAYSECS.flag));
        }
    });
    parent.adopt(Box::new(render_thread));
    parent.adopt(Box::new(collection_thread));
    parent.wait();
    Ok(())
}
