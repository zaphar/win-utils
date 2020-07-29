// Copyright 2020 Jeremy Wall
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use winapi::um::winbase::GetComputerNameW;
use winapi_perf_wrapper::constants;
use winapi_perf_wrapper::*;

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
    let (counters, instances) = pdh
        .enumerate_items_string(obj)
        .map_err(|s| constants::pdh_status_friendly_name(s))
        .unwrap();
    for i in &instances {
        let i = if i.is_empty() {
            String::new()
        } else {
            format!("({})", i)
        };
        for c in &counters {
            // TODO
            println!("\t\\{}{}\\{}", obj, i, c);
        }
    }
}

pub fn print_performance_objects(pdh: &mut PDH) {
    println!("Performance Counter objects:");
    let mut sorted_counters = pdh
        .enumerate_objects_string()
        .map_err(|s| constants::pdh_status_friendly_name(s))
        .unwrap();
    sorted_counters.sort();
    for obj in sorted_counters {
        println!("\t{}", obj);
    }
}

pub fn print_counter_value(pdh: &mut PDH, path: &str) {
    let query = pdh
        .open_query()
        .map_err(|e| constants::pdh_status_friendly_name(e))
        .unwrap();
    let counter_handle = query
        .add_counter_string(path)
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
    //let cpu_counter = r"\\JWALL-SURFACE\Processor information(_Total)\% Processor Time";
    let cpu_counter = r"\\JWALL-SURFACE\Processor Information(_Total)\% Processor Time";
    let query = pdh
        .open_query()
        .map_err(|e| constants::pdh_status_friendly_name(e))
        .unwrap();
    let iterator: CounterStream<i32> = query
        .get_value_stream_from_path(cpu_counter)
        .map_err(|s| constants::pdh_status_friendly_name(s))
        .unwrap()
        .with_delay(std::time::Duration::from_millis(1000));
    //let mem_counter = r"\\JWALL-SURFACE\Memory\Available Bytes";
    loop {
        match iterator.next() {
            Ok(v) => println!("{}: {}", cpu_counter, v),
            Err(s) => eprintln!("Err: {}", constants::pdh_status_friendly_name(s)),
        }
    }
}
