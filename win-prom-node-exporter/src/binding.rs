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

// Tool that owns a query and borrows a registry and sets up the bindings between
// performance counters and prometheus guages.
use lazy_static;
use regex::Regex;

use prometheus::{GaugeVec, Registry};
use winapi_perf_wrapper::constants::pdh_status_friendly_name;
use winapi_perf_wrapper::{CounterStream, PDHStatus, PdhQuery, PDH};

lazy_static::lazy_static! {
    static ref INSTANCE_REGEX: Regex = Regex::new(r".*\((.*)\)").unwrap();
}

fn parse_instance(path: &str) -> String {
    match INSTANCE_REGEX.captures(path) {
        Some(caps) => caps.get(1).unwrap().as_str().to_owned(),
        None => String::new(),
    }
}

fn get_value_stream<'query_life, NumType>(
    query: &'query_life PdhQuery,
    path: &str,
) -> Result<CounterStream<'query_life, NumType>, PDHStatus> {
    query.get_value_stream_from_path::<_, NumType>(path)
}

fn build_metric_pair<'query_life>(
    name: &'static str,
    path: &str,
    registry: &prometheus::Registry,
    query: &'query_life PdhQuery,
) -> anyhow::Result<(&'static str, GaugeVec, CounterStream<'query_life, f64>)> {
    let gauge = GaugeVec::new(prometheus::Opts::new(name, path), &[])?;
    registry.register(Box::new(gauge.clone()))?;
    Ok((
        name,
        gauge,
        get_value_stream::<f64>(query, path)
            .map_err(|s| anyhow::Error::msg(pdh_status_friendly_name(s)))?,
    ))
}

pub struct CounterToPrometheus<'myself, 'registry> {
    pdh: PDH,
    query: PdhQuery,
    registry: &'registry Registry,
    _marker: std::marker::PhantomData<&'myself PdhQuery>,
}

impl<'myself, 'registry> CounterToPrometheus<'myself, 'registry> {
    pub fn try_new(registry: &'registry Registry) -> anyhow::Result<Self> {
        let pdh = PDH::new();
        let query = pdh
            .open_query()
            .map_err(|s| anyhow::Error::msg(pdh_status_friendly_name(s)))?;
        Ok(Self {
            pdh: pdh,
            query: query,
            registry: registry,
            _marker: std::marker::PhantomData,
        })
    }

    pub fn register_pairs(
        &'myself self,
        name_path_pairs: Vec<(&'static str, &str)>,
    ) -> anyhow::Result<Vec<(&str, GaugeVec, CounterStream<'myself, f64>)>> {
        let mut pairs = Vec::new();
        for (name, path) in name_path_pairs {
            let pair = build_metric_pair(name, path, self.registry, &self.query)?;
            pairs.push(pair);
        }
        Ok(pairs)
    }

    pub fn register_wildcard_pairs(
        &'myself self,
        name_path_pairs: Vec<(&'static str, &str)>,
    ) -> anyhow::Result<
        Vec<(
            &str,
            GaugeVec,
            (&'static str, String),
            CounterStream<'myself, f64>,
        )>,
    > {
        let mut pairs = Vec::new();
        for (name, path) in name_path_pairs {
            let expanded_paths = self
                .pdh
                .expand_counter_path_string(path)
                .map_err(|s| anyhow::Error::msg(pdh_status_friendly_name(s)))?;
            for expanded in expanded_paths {
                // TODO(jwall): Parse the instance out first.
                let instance = parse_instance(&expanded);
                let gauge = GaugeVec::new(prometheus::Opts::new(name, &expanded), &["instance"])?;
                self.registry.register(Box::new(gauge.clone()))?;
                pairs.push((
                    name,
                    gauge,
                    ("instance", instance),
                    get_value_stream::<f64>(&self.query, &expanded)
                        .map_err(|s| anyhow::Error::msg(pdh_status_friendly_name(s)))?,
                ));
            }
        }
        Ok(pairs)
    }
}
