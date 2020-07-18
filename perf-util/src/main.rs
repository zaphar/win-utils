use winapi::um::winbase::GetComputerNameW;
use winapi_perf_wrapper::constants;
use winapi_perf_wrapper::PDH;

pub fn print_counters(pdh: &mut PDH) {
    for obj in pdh
        .enumerate_counters()
        .map_err(|e| constants::pdh_status_friendly_name(e))
        .unwrap()
    {
        println!("{}", obj);
    }
}

pub fn print_counter_value(pdh: &mut PDH, path: &str) {
    let mut query = pdh
        .open_query()
        .map_err(|e| constants::pdh_status_friendly_name(e))
        .unwrap();
    let counter_handle = query
        .add_counter(path)
        .map_err(|e| constants::pdh_status_friendly_name(e))
        .unwrap();
    let value = query
        .collect_large_data(&counter_handle)
        .map_err(|e| constants::pdh_status_friendly_name(e))
        .unwrap();
    println!("{}: {}", path, value);
}

fn main() {
    let mut name_size: u32 = 32;
    let mut machine_name = Vec::with_capacity(name_size as usize);
    machine_name.resize(name_size as usize, 0);
    let status = unsafe { GetComputerNameW(machine_name.as_mut_ptr(), &mut name_size) }
        as constants::PDHStatus;
    if status == 0 {
        panic!("Failed to get machine name! error_code {}", status);
    }
    let mut pdh = PDH::new().with_machine_name(String::from_utf16_lossy(machine_name.as_slice()));
    //let cpu_counter = r"\\JWALL-SURFACE\Processor Information(_Total)\% Processor Time";
    let mem_counter = r"\\JWALL-SURFACE\Memory\Available Bytes";
    //println!("Trying counter {}", cpu_counter);
    //print_counter_value(&mut pdh, cpu_counter);
    println!("Trying counter {}", mem_counter);
    print_counter_value(&mut pdh, mem_counter);
    //print_counters(&mut pdh);
}
