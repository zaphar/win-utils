use winapi::um::winbase::GetComputerNameW;
use winapi_perf_wrapper::constants;
use winapi_perf_wrapper::PDH;

pub fn print_counters(pdh: &mut PDH) {
    let mut counter_paths = pdh
        .enumerate_counters()
        .map_err(|e| constants::pdh_status_friendly_name(e))
        .unwrap();
    counter_paths.sort();
    for obj in counter_paths {
        println!("{}", obj);
    }
}

pub fn print_object_counters(pdh: &mut PDH, obj: &str) {
    println!("Counters for {}:", obj);
    let mut obj_utf16 = obj.encode_utf16().collect::<Vec<u16>>();
    obj_utf16.push(0); // We want the machine name to be null terminated.
    let (counters, instances) = pdh
        .enumerate_items(&obj_utf16)
        .map_err(|s| constants::pdh_status_friendly_name(s))
        .unwrap();
    for i in &instances {
        let i = if i.is_empty() {
            String::new()
        } else {
            format!("({})", String::from_utf16_lossy(i))
        };
        for c in &counters {
            // TODO
            println!("\t\\{}{}\\{}", obj, i, String::from_utf16_lossy(c));
        }
    }
}

pub fn print_performance_objects(pdh: &mut PDH) {
    println!("Performance Counter objects:");
    let mut sorted_counters = pdh
        .enumerate_objects()
        .map_err(|s| constants::pdh_status_friendly_name(s))
        .unwrap();
    sorted_counters.sort();
    for obj in sorted_counters {
        println!("\t{}", String::from_utf16_lossy(obj.as_slice()));
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
    // We need our string to be null terminated
    machine_name.resize(name_size as usize, 0);
    println!(
        "MachineName: {}",
        String::from_utf16_lossy(machine_name.as_slice())
    );
    let mut pdh = PDH::new().with_machine_name(machine_name);
    //print_performance_objects(&mut pdh);
    //print_object_counters(&mut pdh, "Processor Information");
    //print_counters(&mut pdh);
    let disk_counter = r"\\JWALL-SURFACE\LogicalDisk(_Total)\% Free Space";
    println!("Trying counter {}", disk_counter);
    print_counter_value(&mut pdh, disk_counter);
    //let cpu_counter = r"\\JWALL-SURFACE\Processor information(_Total)\% Processor Time";
    //println!("Trying counter {}", cpu_counter);
    //print_counter_value(&mut pdh, cpu_counter);
    //let mem_counter = r"\\JWALL-SURFACE\Memory\Available Bytes";
    //println!("Trying counter {}", mem_counter);
    //print_counter_value(&mut pdh, mem_counter);
}
