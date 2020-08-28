use std::convert::Into;
use std::env;
use std::ffi::OsString;
use std::sync::{Mutex, RwLock};
use std::time::Duration;

use anyhow;
use crossbeam_utils::thread;
use docopt;
use eventlog;
use lazy_static;
use log::{debug, error, info};
use prometheus;
use prometheus::Encoder;
use windows_service;
use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use winapi_perf_wrapper::ValueStream;

mod binding;
mod perf_paths;

lazy_static::lazy_static! {
    static ref STOP_SIGNAL: RwLock<bool> = RwLock::new(false);
}

lazy_static::lazy_static! {
    static ref SERVICE_ARGS: std::sync::Mutex<Option<docopt::ArgvMap>> = Mutex::new(None);
}

const SERVICENAME: &'static str = "prom_node_exporter";
const DISPLAYNAME: &'static str = "Prometheus Node Exporter";
const LOGNAME: &'static str = "Prometheus Node Exporter Log";

const USAGE: &'static str = "
Windows Prometheus Node Exporter

Usage: win-prom-node-exporter [options]

Options:
    -h --help            Show this help text
    --delaySecs=S        Delay between collections from windows performance counters in seconds. [default: 10]
    --listenHost=IPPORT  IP and Port combination for the http service to export prometheus metrics on. [default: 0.0.0.0:8080]
    --debug              Enable debug logging.
    --install            Install this windows service with the provided command line flags.
    --remove             Delete this windows service.

    --no-service         Don't run as a Windows Service.
";

fn flags() -> docopt::Docopt {
    docopt::Docopt::new(USAGE).unwrap()
}

fn init_log(argv: &docopt::ArgvMap) -> anyhow::Result<()> {
    if argv.get_bool("--no-service") {
        stderrlog::new()
            .timestamp(stderrlog::Timestamp::Millisecond)
            .verbosity(if argv.get_bool("--debug") { 3 } else { 2 })
            .init()?;
    } else if argv.get_bool("--debug") {
        eventlog::init(LOGNAME, log::Level::Debug)?;
        log::set_max_level(log::LevelFilter::Debug);
    } else {
        eventlog::init(LOGNAME, log::Level::Info)?;
        log::set_max_level(log::LevelFilter::Info);
    }
    Ok(())
}

fn win_service_main(_args: Vec<OsString>) {
    let service_event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                info!("Recieved request to stop service. Setting STOP_SIGNAL to true.");
                let mut guard = STOP_SIGNAL.write().unwrap();
                *guard = true;
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle =
        service_control_handler::register(SERVICENAME, service_event_handler).unwrap();

    let ready_hook = || -> anyhow::Result<()> {
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
        Ok(())
    };

    if let Err(e) = win_service_impl(ready_hook) {
        status_handle
            .set_service_status(ServiceStatus {
                // Should match the one from system service registry
                service_type: ServiceType::OWN_PROCESS,
                // The new state
                current_state: ServiceState::Stopped,
                // Accept no events when running
                controls_accepted: ServiceControlAccept::STOP,
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

        error!("Error starting service: {}", e);
        return;
    }
    status_handle
        .set_service_status(ServiceStatus {
            // Should match the one from system service registry
            service_type: ServiceType::OWN_PROCESS,
            // The new state
            current_state: ServiceState::Stopped,
            // Accept no events when running
            controls_accepted: ServiceControlAccept::STOP,
            // Used to report an error when starting or stopping only, otherwise must be zero
            exit_code: ServiceExitCode::Win32(0),
            // Only used for pending states, otherwise must be zero
            checkpoint: 0,
            // Only used for pending states, otherwise must be zero
            wait_hint: Duration::default(),
            // Unused for setting status
            process_id: None,
        })
        .unwrap(); // if this failed then we are in deep trouble. Just crash.
}

fn win_service_impl<F>(ready_hook: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    let argv = match (*SERVICE_ARGS.lock().unwrap()).clone() {
        Some(argv) => argv,
        None => {
            panic!("Something very unexpected happen. This is a bug.");
        }
    };
    debug!("service_impl args{:?}", argv);
    let registry = prometheus::Registry::new();

    ready_hook()?;

    let listen_host = argv.get_str("--listenHost");
    let delay_secs: u64 = argv.get_count("--delaySecs");

    Ok(thread::scope(|s| {
        s.spawn(|_| {
            info!("Starting server on {}", listen_host);
            let server = tiny_http::Server::http(listen_host).unwrap();
            loop {
                {
                    if *STOP_SIGNAL.read().unwrap() {
                        info!("Stopping prometheus metric server thread.");
                        return;
                    }
                }
                debug!("Waiting for request");
                // NOTE(jwall): We have to not block for longer than the 10 millis to avoid not detecting
                // the stop signal above.
                match server.recv_timeout(std::time::Duration::from_millis(10)) {
                    Ok(Some(req)) => {
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
                    Ok(None) => {
                        // Receive timed out so noop
                    }
                    Err(e) => {
                        error!("Invalid http request! {}", e);
                    }
                }
            }
        });
        s.spawn(|_| {
            debug!("Opening PDH Performance counter query");
            let mut binding = binding::CounterToPrometheus::try_new(&registry).unwrap();
            debug!("Setting up counters and prometheus guages");
            let pairs = binding
                .register_pairs(vec![
                    ("cpu_total_pct", perf_paths::CPU_TOTAL_PCT),
                    ("cpu_user_pct", perf_paths::CPU_USER_PCT),
                    ("cpu_idle_pct", perf_paths::CPU_IDLE_PCT),
                    ("cpu_privileged_pct", perf_paths::CPU_PRIVILEGED_PCT),
                    ("cpu_priority_pct", perf_paths::CPU_PRIORITY_PCT),
                    ("cpu_frequency_gauge", perf_paths::CPU_FREQUENCY),
                    ("mem_available_bytes", perf_paths::MEM_AVAILABLE_BYTES),
                    ("mem_cache_bytes", perf_paths::MEM_CACHE_BYTES),
                    ("mem_committed_bytes", perf_paths::MEM_COMMITTED_BYTES),
                    ("disk_pct_read_time", perf_paths::DISK_PCT_READ_TIME),
                    ("disk_pct_write_time", perf_paths::DISK_PCT_WRITE_TIME),
                    ("disk_read_bytes_sec", perf_paths::DISK_READ_BYTES_SEC),
                    ("disk_write_bytes_sec", perf_paths::DISK_WRITE_BYTES_SEC),
                    ("sys_processes_count", perf_paths::SYS_PROCESSES_COUNT),
                    ("sys_threads_count", perf_paths::SYS_THREADS_COUNT),
                    ("sys_context_switch_sec", perf_paths::SYS_CONTEXT_SWITCH_SEC),
                    ("sys_system_calls_sec", perf_paths::SYS_SYSTEM_CALLS_SEC),
                ])
                .unwrap();
            info!("Starting collection thread");
            loop {
                {
                    if *STOP_SIGNAL.read().unwrap() {
                        info!("Stopping metric collection thread.");
                        return;
                    }
                }
                for (metric, stream) in pairs {
                    if let Ok(v) = stream.next() {
                        metric.with(&prometheus::labels! {}).set(v as f64);
                    }
                }
                debug!("Sleeping until next collection");
                std::thread::sleep(std::time::Duration::from_secs(delay_secs));
            }
        });
    })
    .unwrap())
}

windows_service::define_windows_service!(ffi_service_main, win_service_main);

fn flags_from_argmap(argv: &docopt::ArgvMap) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    if argv.get_bool("--debug") {
        args.push("--debug".into());
    }
    let host = argv.get_str("--listenHost");
    if host != "" {
        args.push("--listenHost".into());
        args.push(host.into());
    }
    let secs = argv.get_str("--delaySecs");
    if secs != "" {
        args.push("--delaySecs".into());
        args.push(secs.into());
    }
    return args;
}

fn main() -> anyhow::Result<()> {
    let docopt = flags();
    let argv = docopt.parse().unwrap_or_else(|e| e.exit());

    {
        // Ensure this is scoped very tightly
        let mut guard = SERVICE_ARGS.lock().unwrap();
        *guard = Some(argv.clone());
    }

    init_log(&argv).unwrap();
    if argv.get_bool("--install") {
        let manager =
            ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;

        let my_service_info = ServiceInfo {
            name: OsString::from(SERVICENAME),
            display_name: OsString::from(DISPLAYNAME),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::OnDemand,
            error_control: ServiceErrorControl::Normal,
            // Derive this from our current path.
            executable_path: env::current_exe()?,
            // Derive this our existing arguments.
            launch_arguments: flags_from_argmap(&argv),
            dependencies: vec![],
            account_name: None, // run as System
            account_password: None,
        };

        manager.create_service(&my_service_info, ServiceAccess::QUERY_STATUS)?;
        eventlog::register(LOGNAME)?;
    } else if argv.get_bool("--remove") {
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::all())?;
        let service = manager.open_service(SERVICENAME, ServiceAccess::DELETE)?;
        service.delete()?;
        eventlog::deregister(LOGNAME)?;
    } else if argv.get_bool("--no-service") {
        win_service_impl(|| Ok(()))?;
    } else {
        windows_service::service_dispatcher::start(SERVICENAME, ffi_service_main)?;
    }
    Ok(())
}
