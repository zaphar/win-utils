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
use anyhow;
use docopt;

use winapi_perf_wrapper::constants;
use winapi_perf_wrapper::*;

const USAGE: &'static str = "
Performance Counter Utility

Usage: perf-util [options]

Options:
    -h --help       Show this help text
    --machine<m>    The MachineName to use
    --expand=<p>    Expand a counter path to its variants
    --stream=<p>    Stream the values for a performance counter
    --list          List available counters
";

pub fn print_counters(pdh: &mut PDH) -> anyhow::Result<()> {
    let mut counter_paths = pdh
        .enumerate_counters()
        .map_err(|e| constants::pdh_status_friendly_name(e))
        .unwrap();
    counter_paths.sort();
    for obj in counter_paths {
        println!("{}", obj);
    }
    Ok(())
}

pub fn print_object_counters(pdh: &mut PDH, obj: &str) -> anyhow::Result<()> {
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
    Ok(())
}

pub fn print_performance_objects(pdh: &mut PDH) -> anyhow::Result<()> {
    println!("Performance Counter objects:");
    let mut sorted_counters = pdh
        .enumerate_objects_string()
        .map_err(|s| constants::pdh_status_friendly_name(s))
        .unwrap();
    sorted_counters.sort();
    for obj in sorted_counters {
        println!("\t{}", obj);
    }
    Ok(())
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

fn main() -> anyhow::Result<()> {
    let parser = docopt::Docopt::new(USAGE)?;
    let argv = parser.parse()?;
    let mut pdh = if let Some(machine) = argv.find("--machine") {
        PDH::new().with_machine_name(machine.as_str().encode_utf16().collect())
    } else {
        PDH::new()
    };

    if argv.get_bool("--list") {
        print_counters(&mut pdh)?;
    } else if argv.get_str("--expand") != "" {
        let path = argv.get_str("--expand");
        let paths = pdh
            .expand_counter_path_string(path)
            .map_err(|e| constants::pdh_status_friendly_name(e))
            .unwrap();
        for p in paths {
            println!("{}", p);
        }
    } else if argv.get_str("--stream") != "" {
        let path = argv.get_str("--stream");
        let query = pdh
            .open_query()
            .map_err(|e| constants::pdh_status_friendly_name(e))
            .unwrap();
        let iterator: CounterStream<i32> = query
            .get_value_stream_from_path(path)
            .map_err(|s| constants::pdh_status_friendly_name(s))
            .unwrap()
            .with_delay(std::time::Duration::from_millis(1000));
        // Throw away the first value. It will always be garbage.
        let _ = iterator.next();
        loop {
            match iterator.next() {
                Ok(v) => println!("{}\t{}", path, v),
                Err(s) => eprintln!("Err: {}", constants::pdh_status_friendly_name(s)),
            }
        }
    }
    Ok(())
}
