use std::ffi::OsString;
use std::time::Duration;

use anyhow;
use crossbeam_utils::thread;
use gflags;
use log::{debug, error, info};
use prometheus;
use prometheus::Encoder;
use winapi_perf_wrapper::{CounterStream, PDHStatus, PdhQuery, ValueStream, PDH};
use windows_service;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{
    self, ServiceControlHandlerResult, ServiceStatusHandle,
};

mod perf_paths;

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

gflags::define! {
    /// Windows Service Name
    --serviceName = "win_prom_node_exporter"
}

fn usage(code: i32) {
    println!("win-prom-node-exporter <flags>");
    println!("");
    gflags::print_help_and_exit(code);
}

fn get_value_stream<'query_life, NumType>(
    query: &'query_life PdhQuery,
    path: &str,
) -> Result<CounterStream<'query_life, NumType>, PDHStatus> {
    query.get_value_stream_from_path::<_, NumType>(path)
}

fn win_service_main(args: Vec<OsString>) {
    let service_event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => {
                // TODO correctly handle the stop event.
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle =
        service_control_handler::register(SERVICENAME.flag, service_event_handler).unwrap();

    if let Err(e) = win_service_wrapper(args, &status_handle) {
        status_handle
            .set_service_status(ServiceStatus {
                // Should match the one from system service registry
                service_type: ServiceType::OWN_PROCESS,
                // The new state
                current_state: ServiceState::Stopped,
                // Accept no events when running
                controls_accepted: ServiceControlAccept::empty(),
                // Used to report an error when starting or stopping only, otherwise must be zero
                exit_code: ServiceExitCode::Win32(1),
                // Only used for pending states, otherwise must be zero
                checkpoint: 0,
                // Only used for pending states, otherwise must be zero
                wait_hint: Duration::default(),
                // Unused for setting status
                process_id: None,
            })
            .unwrap(); // if this failed then we are in deep trouble. Just crash.

        eprintln!("Error starting service: {}", e);
    }
}

fn win_service_wrapper(
    _args: Vec<OsString>,
    status_handle: &ServiceStatusHandle,
) -> anyhow::Result<()> {
    let prom_cpu_pct_gauge = prometheus::GaugeVec::new(
        prometheus::Opts::new("cpu_total_pct", perf_paths::CPU_TOTAL_PCT),
        &[],
    )?;
    let prom_cpu_idle_gauge = prometheus::GaugeVec::new(
        prometheus::Opts::new("cpu_idle_pct", perf_paths::CPU_IDLE_PCT),
        &[],
    )?;
    let prom_cpu_user_gauge = prometheus::GaugeVec::new(
        prometheus::Opts::new("cpu_user_pct", perf_paths::CPU_USER_PCT),
        &[],
    )?;
    let prom_cpu_privileged_gauge = prometheus::GaugeVec::new(
        prometheus::Opts::new("cpu_privileged_pct", perf_paths::CPU_PRIVILEGED_PCT),
        &[],
    )?;
    let prom_cpu_priority_gauge = prometheus::GaugeVec::new(
        prometheus::Opts::new("cpu_priority_pct", perf_paths::CPU_PRIORITY_PCT),
        &[],
    )?;
    let prom_cpu_frequency_guage = prometheus::GaugeVec::new(
        prometheus::Opts::new("cpu_frequency_gauge", perf_paths::CPU_FREQUENCY),
        &[],
    )?;
    let prom_mem_available_guage = prometheus::GaugeVec::new(
        prometheus::Opts::new("mem_available_bytes", perf_paths::MEM_AVAILABLE_BYTES),
        &[],
    )?;
    let prom_mem_cache_guage = prometheus::GaugeVec::new(
        prometheus::Opts::new("mem_cache_bytes", perf_paths::MEM_CACHE_BYTES),
        &[],
    )?;
    let prom_mem_committed_guage = prometheus::GaugeVec::new(
        prometheus::Opts::new("mem_committed_bytes", perf_paths::MEM_COMMITTED_BYTES),
        &[],
    )?;
    debug!("Setting up registry of prometheus metrics");
    let registry = prometheus::Registry::new();
    registry.register(Box::new(prom_cpu_pct_gauge.clone()))?;
    registry.register(Box::new(prom_cpu_user_gauge.clone()))?;
    registry.register(Box::new(prom_cpu_idle_gauge.clone()))?;
    registry.register(Box::new(prom_cpu_frequency_guage.clone()))?;
    registry.register(Box::new(prom_cpu_privileged_gauge.clone()))?;
    registry.register(Box::new(prom_cpu_priority_gauge.clone()))?;
    registry.register(Box::new(prom_mem_available_guage.clone()))?;
    registry.register(Box::new(prom_mem_cache_guage.clone()))?;
    registry.register(Box::new(prom_mem_committed_guage.clone()))?;

    status_handle.set_service_status(ServiceStatus {
        // Should match the one from system service registry
        service_type: ServiceType::OWN_PROCESS,
        // The new state
        current_state: ServiceState::Running,
        // Accept stop events when running
        controls_accepted: ServiceControlAccept::STOP,
        // Used to report an error when starting or stopping only, otherwise must be zero
        exit_code: ServiceExitCode::Win32(0),
        // Only used for pending states, otherwise must be zero
        checkpoint: 0,
        // Only used for pending states, otherwise must be zero
        wait_hint: Duration::default(),
        // Unused for setting status
        process_id: None,
    })?;

    Ok(thread::scope(|s| {
        s.spawn(|_| {
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
        s.spawn(|_| {
            debug!("Opening PDH Performance counter query");
            let mut pdh = PDH::new();
            let query = pdh.open_query().unwrap();
            debug!("Adding counters to query");
            let cpu_total_stream =
                get_value_stream::<f64>(&query, perf_paths::CPU_TOTAL_PCT).unwrap();
            let cpu_idle_stream =
                get_value_stream::<f64>(&query, perf_paths::CPU_IDLE_PCT).unwrap();
            let cpu_user_stream =
                get_value_stream::<f64>(&query, perf_paths::CPU_USER_PCT).unwrap();
            let cpu_priority_stream =
                get_value_stream::<f64>(&query, perf_paths::CPU_PRIORITY_PCT).unwrap();
            let cpu_privileged_stream =
                get_value_stream::<f64>(&query, perf_paths::CPU_PRIVILEGED_PCT).unwrap();
            let cpu_frequency_stream =
                get_value_stream::<f64>(&query, perf_paths::CPU_FREQUENCY).unwrap();
            let mem_available_stream =
                get_value_stream::<f64>(&query, perf_paths::MEM_AVAILABLE_BYTES).unwrap();
            let mem_cache_stream =
                get_value_stream::<f64>(&query, perf_paths::MEM_CACHE_BYTES).unwrap();
            let mem_committed_stream =
                get_value_stream::<f64>(&query, perf_paths::MEM_COMMITTED_BYTES).unwrap();
            info!("Starting collection thread");
            loop {
                if let Ok(v) = cpu_total_stream.next() {
                    prom_cpu_pct_gauge
                        .with(&prometheus::labels! {})
                        .set(v as f64);
                }
                if let Ok(v) = cpu_idle_stream.next() {
                    prom_cpu_idle_gauge
                        .with(&prometheus::labels! {})
                        .set(v as f64);
                }
                if let Ok(v) = cpu_user_stream.next() {
                    prom_cpu_user_gauge
                        .with(&prometheus::labels! {})
                        .set(v as f64);
                }
                if let Ok(v) = cpu_privileged_stream.next() {
                    prom_cpu_privileged_gauge
                        .with(&prometheus::labels! {})
                        .set(v as f64);
                }
                if let Ok(v) = cpu_priority_stream.next() {
                    prom_cpu_priority_gauge
                        .with(&prometheus::labels! {})
                        .set(v as f64);
                }
                if let Ok(v) = cpu_frequency_stream.next() {
                    prom_cpu_frequency_guage
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
    })
    .unwrap())
}

windows_service::define_windows_service!(ffi_service_main, win_service_main);

fn main() -> anyhow::Result<()> {
    gflags::parse();
    if HELP.flag {
        usage(0);
    }

    let level = if DEBUG.flag || cfg!(debug_assertions) {
        2
    } else {
        3
    };
    stderrlog::new()
        .verbosity(level)
        .timestamp(stderrlog::Timestamp::Millisecond)
        .init()?;
    windows_service::service_dispatcher::start(SERVICENAME.flag, ffi_service_main)?;
    Ok(())
}
