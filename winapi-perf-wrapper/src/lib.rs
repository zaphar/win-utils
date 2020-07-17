use winapi::ctypes::c_void;
use winapi::shared::minwindef::{DWORD, FALSE, TRUE};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::pdh::{
    PdhCloseQuery, PdhEnumObjectItemsW, PdhEnumObjectsW, PdhOpenQueryW, PDH_HQUERY as HQuery,
    PERF_DETAIL_STANDARD,
};

use std::ffi::{OsStr, OsString};
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
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
            format!("/{}", String::from_utf16_lossy(machine_name.as_slice()))
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

    // TODO Add counter?

    // TODO Counter data interator?
}

impl Drop for PdhQuery {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.0);
        }
    }
}
