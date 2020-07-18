use winapi::shared::minwindef::{DWORD, FALSE, TRUE};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::pdh::{
    PDH_FMT_COUNTERVALUE_u, PdhAddCounterW, PdhCloseQuery, PdhCollectQueryData,
    PdhEnumObjectItemsW, PdhEnumObjectsW, PdhGetFormattedCounterValue, PdhOpenQueryW,
    PdhRemoveCounter, PdhValidatePathW, PDH_FMT_COUNTERVALUE, PDH_HCOUNTER as HCounter,
    PDH_HQUERY as HQuery, PERF_DETAIL_STANDARD,
};

use std::ptr::null_mut;

pub mod constants;
use constants::*;

pub struct PDH {
    // TODO(jwall): Do we need interior mutability here?
    /// If None then use localhost. If set then use that machine_name.
    machine_name: Option<Vec<u16>>,
}

fn null_separated_to_vec(mut buf: Vec<u16>) -> Vec<String> {
    // The buffer is terminated by two nulls so we pop the last two off
    // for our partition below to work.
    buf.pop();
    buf.pop();
    let mut v = Vec::new();
    for item in buf.split(|el| *el == 0) {
        v.push(String::from_utf16_lossy(item));
    }
    return v;
}

fn str_to_utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn zeroed_buffer(sz: usize) -> Vec<u16> {
    let mut v = Vec::with_capacity(sz);
    v.resize(sz, Default::default());
    return v;
}

impl PDH {
    pub fn new() -> Self {
        Self { machine_name: None }
    }

    pub fn with_machine_name<S: Into<String>>(mut self, machine_name: S) -> Self {
        self.machine_name = Some(machine_name.into().encode_utf16().collect());
        self
    }

    fn enumerate_objects(&mut self) -> Result<Vec<String>, PDHStatus> {
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

    fn enumerate_items(&mut self, obj: &str) -> Result<(Vec<String>, Vec<String>), PDHStatus> {
        let mut object_name = str_to_utf16(&obj);
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

    pub fn open_query(&mut self) -> Result<PdhQuery, PDHStatus> {
        let mut query = PdhQuery(null_mut());
        let status = unsafe { PdhOpenQueryW(null_mut(), 0, query.query()) } as u32;

        if status != ERROR_SUCCESS {
            return Err(status);
        }
        return Ok(query);
    }

    pub fn enumerate_counters(&mut self) -> Result<Vec<String>, PDHStatus> {
        let mut counter_path_vec = Vec::new();
        let path_prefix = if let Some(ref machine_name) = self.machine_name {
            format!("\\\\{}", String::from_utf16_lossy(machine_name.as_slice()))
        } else {
            String::new()
        };
        for obj in self.enumerate_objects()? {
            let (counters, instances) = match self.enumerate_items(&obj) {
                Ok(t) => t,
                Err(PDH_CSTATUS_NO_OBJECT) => {
                    continue;
                }
                Err(s) => return Err(s),
            };
            for i in &instances {
                let i = if i == "" {
                    String::new()
                } else {
                    format!("({})", i)
                };
                for c in &counters {
                    // TODO
                    counter_path_vec.push(format!("{}\\{}{}\\{}", path_prefix, obj, i, c));
                }
            }
        }
        return Ok(counter_path_vec);
    }
}

pub struct PdhQuery(HQuery);

impl PdhQuery {
    pub fn query(&mut self) -> &mut HQuery {
        &mut self.0
    }

    pub fn add_counter<S: Into<String>>(&mut self, path: S) -> Result<PdhCounter, PDHStatus> {
        let wide_path = path.into().encode_utf16().collect::<Vec<u16>>();
        let mut status = unsafe { PdhValidatePathW(wide_path.as_ptr()) } as u32;
        if status != ERROR_SUCCESS {
            return Err(dbg!(status));
        }
        let mut counter_handle: HCounter = null_mut();
        status =
            unsafe { PdhAddCounterW(self.0, wide_path.as_ptr(), 0, &mut counter_handle) } as u32;
        if status != ERROR_SUCCESS {
            return Err(status);
        }
        return Ok(PdhCounter(counter_handle));
    }

    #[allow(unused_variables)]
    pub fn remove_counter(&self, counter_handle: PdhCounter) {
        // when the counter is dropped it will be removed from the query.
        // We consume the counter in the process so it can't be reused.
        // As such this function has not body. It exists only to consume the counter.
    }

    pub fn collect_data(
        &self,
        counter: &PdhCounter,
        format: u32,
    ) -> Result<PDH_FMT_COUNTERVALUE, PDHStatus> {
        let mut status = unsafe { PdhCollectQueryData(self.0) } as u32;
        if status != ERROR_SUCCESS {
            return Err(dbg!(status));
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
            return Err(dbg!(status));
        }
        return Ok(fmt_counter_value);
    }

    pub fn collect_long_data(&self, counter: &PdhCounter) -> Result<i32, PDHStatus> {
        let fmt_counter_value = self.collect_data(counter, PDH_FMT_LONG)?;
        return Ok(unsafe { *fmt_counter_value.u.longValue() });
    }

    pub fn collect_large_data(&self, counter: &PdhCounter) -> Result<i64, PDHStatus> {
        let fmt_counter_value = self.collect_data(counter, PDH_FMT_LARGE)?;
        return Ok(unsafe { *fmt_counter_value.u.largeValue() });
    }

    pub fn collect_double_data(&self, counter: &PdhCounter) -> Result<f64, PDHStatus> {
        let fmt_counter_value = self.collect_data(counter, PDH_FMT_DOUBLE)?;
        return Ok(unsafe { *fmt_counter_value.u.doubleValue() });
    }
}

impl Drop for PdhQuery {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.0);
        }
    }
}

pub struct PdhCounter(HCounter);

impl Drop for PdhCounter {
    fn drop(&mut self) {
        unsafe {
            PdhRemoveCounter(self.0);
        }
    }
}
