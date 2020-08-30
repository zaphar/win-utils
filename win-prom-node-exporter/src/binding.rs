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

use prometheus::{GaugeVec, Registry};
use winapi_perf_wrapper::constants::pdh_status_friendly_name;
use winapi_perf_wrapper::{CounterStream, PDHStatus, PdhQuery, PDH};

fn get_value_stream<'query_life, NumType>(
    query: &'query_life PdhQuery,
    path: &str,
) -> Result<CounterStream<'query_life, NumType>, PDHStatus> {
    query.get_value_stream_from_path::<_, NumType>(path)
}

fn build_metric_pair<'query_life>(
    name: &str,
    path: &str,
    registry: &prometheus::Registry,
    query: &'query_life PdhQuery,
) -> anyhow::Result<(GaugeVec, CounterStream<'query_life, f64>)> {
    let gauge = GaugeVec::new(prometheus::Opts::new(name, path), &[])?;
    registry.register(Box::new(gauge.clone()))?;
    Ok((
        gauge,
        get_value_stream::<f64>(query, path)
            .map_err(|s| anyhow::Error::msg(pdh_status_friendly_name(s)))?,
    ))
}

pub struct CounterToPrometheus<'myself, 'registry> {
    query: PdhQuery,
    registry: &'registry Registry,
    pairs: Vec<(GaugeVec, CounterStream<'myself, f64>)>,
}

impl<'myself, 'registry> CounterToPrometheus<'myself, 'registry> {
    pub fn try_new(registry: &'registry Registry) -> anyhow::Result<Self> {
        Ok(Self {
            query: PDH::new()
                .open_query()
                .map_err(|s| anyhow::Error::msg(pdh_status_friendly_name(s)))?,
            registry: registry,
            pairs: Vec::new(),
        })
    }

    pub fn register_pairs(
        &'myself mut self,
        name_path_pairs: Vec<(&str, &str)>,
    ) -> anyhow::Result<&Vec<(GaugeVec, CounterStream<'myself, f64>)>> {
        for (name, path) in name_path_pairs {
            let pair = build_metric_pair(name, path, self.registry, &self.query)?;
            self.pairs.push(pair);
        }
        Ok(&self.pairs)
    }
}
