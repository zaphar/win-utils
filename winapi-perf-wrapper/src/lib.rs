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
//! Ergonomic wrappers around the PDH api for performance counters in windows.
//! This does not support the full PDH API as of yet and is focused on making
//! reading existing counters easier not creating custom counters yet.
//! We may add that capability at a later date.
use winapi::shared::minwindef::{DWORD, FALSE, TRUE};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::pdh::{
    PDH_FMT_COUNTERVALUE_u, PdhAddCounterW, PdhCloseQuery, PdhCollectQueryData,
    PdhEnumObjectItemsW, PdhEnumObjectsW, PdhExpandCounterPathW, PdhGetFormattedCounterValue,
    PdhOpenQueryW, PdhRemoveCounter, PdhValidatePathW, PDH_FMT_COUNTERVALUE,
    PDH_HCOUNTER as HCounter, PDH_HQUERY as HQuery, PERF_DETAIL_STANDARD,
};

use std::ptr::null_mut;
use std::time::Duration;

pub mod constants;
pub use constants::PDHStatus;
use constants::*;

fn null_separated_to_vec(mut buf: Vec<u16>) -> Vec<Vec<u16>> {
    // The buffer is terminated by two nulls so we pop the last two off
    // for our partition below to work.
    buf.pop();
    buf.pop();
    let mut v = Vec::new();
    for item in buf.split(|el| *el == 0) {
        v.push(item.to_owned());
    }
    return v;
}

fn str_to_utf16(s: &str) -> Vec<u16> {
    let mut v = s.encode_utf16().collect::<Vec<u16>>();
    v.push(0);
    v
}

fn zeroed_buffer(sz: usize) -> Vec<u16> {
    let mut v = Vec::with_capacity(sz);
    v.resize(sz, Default::default());
    return v;
}

/// PDH api integration for an optional machine name.
pub struct PDH {
    // TODO(jwall): Do we need interior mutability here?
    /// If None then use localhost. If set then use that machine_name.
    machine_name: Option<Vec<u16>>,
}

impl PDH {
    /// Constructs a new PDH instance.
    pub fn new() -> Self {
        Self { machine_name: None }
    }

    /// Sets the machine name for this PDH instance.
    pub fn with_machine_name(mut self, machine_name: Vec<u16>) -> Self {
        self.machine_name = Some(machine_name);
        // We need our machine_name to be a null terminated string.
        self
    }

    /// Enumerates the counter objects for the provided machine or the local machine.
    pub fn enumerate_objects_string(&mut self) -> Result<Vec<String>, PDHStatus> {
        self.enumerate_objects_utf16().map(|mut v| {
            v.drain(0..)
                .map(|v| String::from_utf16_lossy(v.as_slice()))
                .collect()
        })
    }

    /// Enumerates the counter objects for the provided machine or the local machine.
    pub fn enumerate_objects_utf16(&mut self) -> Result<Vec<Vec<u16>>, PDHStatus> {
        let data_source = null_mut();
        let machine_name = if let Some(ref mut machine_name) = self.machine_name {
            machine_name.as_mut_ptr()
        } else {
            null_mut()
        };
        let mut buffer_length: DWORD = 0;
        // The first time we call this to find out what the required buffer
        // size is.
        let mut status = unsafe {
            PdhEnumObjectsW(
                data_source,
                machine_name,
                null_mut(),
                &mut buffer_length,
                PERF_DETAIL_STANDARD,
                TRUE,
            )
        } as u32;
        if status == constants::PDH_MORE_DATA {
            // buffer length should be set to the appropriate length.
            // Now call it a second time to get the list of objects.
            // This will be filled with a null separated list of names.
            let mut object_list = Vec::<u16>::with_capacity(buffer_length as usize);
            object_list.resize(buffer_length as usize, 0);
            status = unsafe {
                PdhEnumObjectsW(
                    data_source,
                    machine_name,
                    object_list.as_mut_ptr(),
                    &mut buffer_length,
                    PERF_DETAIL_STANDARD,
                    FALSE,
                )
            } as u32;
            if status == ERROR_SUCCESS {
                return Ok(null_separated_to_vec(object_list));
            } else {
                return Err(status);
            }
        } else {
            // Error! we expected more data here.
            return Err(status);
        }
    }

    /// Enumerates the objects counter items for the provided machine or the local machine.
    /// Returns a tuple of (counters, instances) for each of those counters.
    pub fn enumerate_items_string<S: Into<String>>(
        &self,
        obj: S,
    ) -> Result<(Vec<String>, Vec<String>), PDHStatus> {
        self.enumerate_items_utf16(&str_to_utf16(&obj.into()))
            .map(|(mut cs, mut insts)| {
                (
                    cs.drain(0..)
                        .map(|v| String::from_utf16_lossy(v.as_slice()))
                        .collect(),
                    insts
                        .drain(0..)
                        .map(|v| String::from_utf16_lossy(v.as_slice()))
                        .collect(),
                )
            })
    }

    /// Enumerates the objects counter items for the provided machine or the local machine.
    /// Returns a tuple of (counters, instances) for each of those counters.
    pub fn enumerate_items_utf16(
        &self,
        obj: &Vec<u16>,
    ) -> Result<(Vec<Vec<u16>>, Vec<Vec<u16>>), PDHStatus> {
        let mut object_name = obj.clone();
        let mut counter_list_len: DWORD = 0;
        let mut instance_list_len: DWORD = 0;
        let mut status = unsafe {
            PdhEnumObjectItemsW(
                null_mut(),
                null_mut(),
                object_name.as_mut_ptr(),
                null_mut(),
                &mut counter_list_len,
                null_mut(),
                &mut instance_list_len,
                PERF_DETAIL_STANDARD,
                0,
            )
        } as PDHStatus;
        if status == constants::PDH_MORE_DATA {
            let mut counter_list = zeroed_buffer(counter_list_len as usize);
            let mut instance_list = zeroed_buffer(instance_list_len as usize);
            status = unsafe {
                PdhEnumObjectItemsW(
                    null_mut(),
                    null_mut(),
                    object_name.as_mut_ptr(),
                    counter_list.as_mut_ptr(),
                    &mut counter_list_len,
                    instance_list.as_mut_ptr(),
                    &mut instance_list_len,
                    PERF_DETAIL_STANDARD,
                    0,
                )
            } as PDHStatus;
            if status != ERROR_SUCCESS {
                return Err(status);
            }
            return Ok((
                null_separated_to_vec(counter_list),
                null_separated_to_vec(instance_list),
            ));
        } else {
            return Err(status);
        }
    }

    /// Opens a query for the configured machine or the local machine.
    pub fn open_query(&self) -> Result<PdhQuery, PDHStatus> {
        let mut query = PdhQuery(null_mut());
        let status = unsafe { PdhOpenQueryW(null_mut(), 0, query.query()) } as u32;

        if status != ERROR_SUCCESS {
            return Err(status);
        }
        return Ok(query);
    }

    /// Enumerates all of the counter paths on the configured machien or local machine.
    pub fn enumerate_counters(&mut self) -> Result<Vec<String>, PDHStatus> {
        let mut counter_path_vec = Vec::new();
        let path_prefix = if let Some(ref machine_name) = self.machine_name {
            // First we need to pad the machine name with null bytes.
            format!("\\\\{}", String::from_utf16_lossy(machine_name.as_slice()))
        } else {
            String::new()
        };
        for obj in self.enumerate_objects_utf16()? {
            let (counters, instances) = match self.enumerate_items_utf16(&obj) {
                Ok(t) => t,
                Err(PDH_CSTATUS_NO_OBJECT) => {
                    continue;
                }
                Err(s) => return Err(s),
            };
            let obj = String::from_utf16_lossy(obj.as_slice());
            for i in &instances {
                let i = if i.is_empty() {
                    String::new()
                } else {
                    format!("({})", String::from_utf16_lossy(i))
                };
                for c in &counters {
                    // TODO
                    counter_path_vec.push(format!(
                        "{}\\{}{}\\{}",
                        path_prefix,
                        obj,
                        i,
                        String::from_utf16_lossy(c)
                    ));
                }
            }
        }
        return Ok(counter_path_vec);
    }

    pub fn expand_counter_path_utf16(&self, path: &Vec<u16>) -> Result<Vec<Vec<u16>>, PDHStatus> {
        let mut counter_list_len: DWORD = 0;
        let mut status =
            unsafe { PdhExpandCounterPathW(path.as_ptr(), null_mut(), &mut counter_list_len) }
                as PDHStatus;
        if status != ERROR_SUCCESS {
            return Err(status);
        }
        let mut unparsed_list = zeroed_buffer(counter_list_len as usize);
        status = unsafe {
            PdhExpandCounterPathW(
                path.as_ptr(),
                unparsed_list.as_mut_ptr(),
                &mut counter_list_len,
            )
        } as PDHStatus;
        if status != ERROR_SUCCESS {
            return Err(status);
        }
        Ok(null_separated_to_vec(unparsed_list))
    }

    pub fn expand_counter_path_string<S: Into<String>>(
        &self,
        path: S,
    ) -> Result<Vec<String>, PDHStatus> {
        self.expand_counter_path_utf16(&str_to_utf16(&path.into()))
            .map(|mut ps| {
                ps.drain(0..)
                    .map(|v| String::from_utf16_lossy(v.as_slice()))
                    .collect()
            })
    }
}

/// A handle for a PDH Query. Queries can have multiple associated PdhCounters.
pub struct PdhQuery(HQuery);

impl PdhQuery {
    /// Convenience query accessor
    pub fn query(&mut self) -> &mut HQuery {
        &mut self.0
    }

    /// Adds a performance counter for the given path in utf16 format.
    pub fn add_counter_utf16(&self, wide_path: Vec<u16>) -> Result<PdhCounter, PDHStatus> {
        let mut status = unsafe { PdhValidatePathW(wide_path.as_ptr()) } as u32;
        if status != ERROR_SUCCESS {
            return Err(status);
        }
        let mut counter_handle: HCounter = null_mut();
        status =
            unsafe { PdhAddCounterW(self.0, wide_path.as_ptr(), 0, &mut counter_handle) } as u32;
        if status != ERROR_SUCCESS {
            return Err(status);
        }
        return Ok(PdhCounter(counter_handle));
    }

    /// Adds a performance counter for the given path.
    pub fn add_counter_string<S: Into<String>>(&self, path: S) -> Result<PdhCounter, PDHStatus> {
        self.add_counter_utf16(str_to_utf16(&path.into()))
    }

    /// Removes a counter from the query consuming it in the process.
    #[allow(unused_variables)]
    pub fn remove_counter(&self, counter_handle: PdhCounter) {
        // when the counter is dropped it will be removed from the query.
        // We consume the counter in the process so it can't be reused.
        // As such this function has no body. It exists only to consume the counter.
    }

    fn collect_data(
        &self,
        counter: &PdhCounter,
        format: u32,
    ) -> Result<PDH_FMT_COUNTERVALUE, PDHStatus> {
        let mut status = unsafe { PdhCollectQueryData(self.0) } as u32;
        if status != ERROR_SUCCESS {
            return Err(status);
        }
        let mut fmt_counter_value = unsafe {
            PDH_FMT_COUNTERVALUE {
                CStatus: 0,
                u: std::mem::zeroed::<PDH_FMT_COUNTERVALUE_u>(),
            }
        };
        let mut counter_type: u32 = 0;
        status = unsafe {
            PdhGetFormattedCounterValue(
                counter.0,
                format,
                &mut counter_type,
                &mut fmt_counter_value,
            )
        } as u32;
        if status != ERROR_SUCCESS {
            return Err(status);
        }
        return Ok(fmt_counter_value);
    }

    /// Returns a ValueStream for a given path that will iterate over
    /// the counter values forever.
    pub fn get_value_stream_from_path<S: Into<String>, ValueType>(
        &self,
        counter_path: S,
    ) -> Result<CounterStream<ValueType>, PDHStatus> {
        let counter_handle = self.add_counter_string(counter_path)?;
        Ok(self.get_value_stream_from_handle(counter_handle))
    }

    /// Returns a ValueStream for a given PdhCounter that will iterator
    /// over the counter values forever.
    /// The PdhCounter must be associated with this query or the iterator
    /// will return errors always.
    pub fn get_value_stream_from_handle<ValueType>(
        &self,
        counter: PdhCounter,
    ) -> CounterStream<ValueType> {
        CounterStream::new(self, counter)
    }

    /// Collect data from a counter in i32 format.
    /// The PdhCounter must be associated with this query.
    pub fn collect_long_data(&self, counter: &PdhCounter) -> Result<i32, PDHStatus> {
        let fmt_counter_value = self.collect_data(counter, PDH_FMT_LONG)?;
        return Ok(unsafe { *fmt_counter_value.u.longValue() });
    }

    /// Collect data from a counter in i64 format.
    /// The PdhCounter must be associated with this query.
    pub fn collect_large_data(&self, counter: &PdhCounter) -> Result<i64, PDHStatus> {
        let fmt_counter_value = self.collect_data(counter, PDH_FMT_LARGE)?;
        return Ok(unsafe { *fmt_counter_value.u.largeValue() });
    }

    /// Collect data from a counter in f64 format.
    /// The PdhCounter must be associated with this query.
    pub fn collect_double_data(&self, counter: &PdhCounter) -> Result<f64, PDHStatus> {
        let fmt_counter_value = self.collect_data(counter, PDH_FMT_DOUBLE)?;
        return Ok(unsafe { *fmt_counter_value.u.doubleValue() });
    }
}

/// Represents a stream of Values or Errors for a given ValueType.
/// (i.e. i32, i64, or f64). Calling next will return the next value
/// for the counter or a Err(PDHStatus).
///
/// Note that an Err return from next does not imply that the stream
/// has ended. Subsequent calls may succeed.
pub trait ValueStream<ValueType> {
    fn next(&self) -> Result<ValueType, PDHStatus>;
}

/// An iterator for a given ValueType over a PdhCounter.
///
/// Note that sometimes the first value returned from a windows performance
/// counter query is invalid but that subsequent values will then be okay.
pub struct CounterStream<'a, ValueType> {
    query_handle: &'a PdhQuery,
    counter_handle: PdhCounter,
    collect_delay: Option<Duration>,
    phantom: std::marker::PhantomData<ValueType>,
}

impl<'a, ValueType> CounterStream<'a, ValueType> {
    /// Constructs a new CounterStream from a PdhQuery and a PdhCounter.
    pub fn new<'b: 'a>(query_handle: &'b PdhQuery, counter_handle: PdhCounter) -> Self {
        Self {
            query_handle: query_handle,
            counter_handle: counter_handle,
            phantom: std::marker::PhantomData,
            collect_delay: None,
        }
    }

    /// Add an optional delay to the iterator. This is useful for when
    /// you want to ensure that you don't spam the counter collection.
    /// Collecting too quickly will yield garbage data from your counter.
    pub fn with_delay<D: Into<Duration>>(mut self, delay: D) -> Self {
        self.collect_delay = Some(delay.into());
        return self;
    }
}

impl<'a> ValueStream<i32> for CounterStream<'a, i32> {
    fn next(&self) -> Result<i32, PDHStatus> {
        if let Some(d) = self.collect_delay {
            std::thread::sleep(d);
        }
        self.query_handle.collect_long_data(&self.counter_handle)
    }
}

impl<'a> ValueStream<i64> for CounterStream<'a, i64> {
    fn next(&self) -> Result<i64, PDHStatus> {
        if let Some(d) = self.collect_delay {
            std::thread::sleep(d);
        }
        self.query_handle.collect_large_data(&self.counter_handle)
    }
}

impl<'a> ValueStream<f64> for CounterStream<'a, f64> {
    fn next(&self) -> Result<f64, PDHStatus> {
        if let Some(d) = self.collect_delay {
            std::thread::sleep(d);
        }
        self.query_handle.collect_double_data(&self.counter_handle)
    }
}

impl Drop for PdhQuery {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.0);
        }
    }
}

/// A wrapper for the PDH counter handle provided by a query object when
/// you add a counter.
pub struct PdhCounter(HCounter);

impl Drop for PdhCounter {
    fn drop(&mut self) {
        unsafe {
            PdhRemoveCounter(self.0);
        }
    }
}
