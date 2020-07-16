use winapi_perf_wrapper::constants;
use winapi_perf_wrapper::PDH;

fn main() {
    let mut pdh = PDH::new();
    for obj in pdh
        .enumerate_counters()
        .map_err(|e| constants::pdh_status_friendly_name(e))
        .unwrap()
    {
        println!("{}", obj);
    }
}
