#![allow(dead_code)]

pub type PDHStatus = u32;

// pdh.h
pub const PDH_MAX_COUNTER_PATH: PDHStatus = 2048;
pub const PDH_MAX_COUNTER_NAME: PDHStatus = 1024;
pub const PDH_MAX_INSTANCE_NAME: PDHStatus = 1024;
pub const PDH_MAX_DATASOURCE_PATH: PDHStatus = 1024;
// pdhmsg.h
pub const PDH_DIALOG_CANCELLED: PDHStatus = 0x800007D9;
pub const PDH_CSTATUS_NO_OBJECT: PDHStatus = 0xC0000BB8;
pub const PDH_CSTATUS_NO_MACHINE: PDHStatus = 0x800007D0;
pub const PDH_MORE_DATA: PDHStatus = 0x800007D2;
pub const PDH_MEMORY_ALLOCATION_FAILURE: PDHStatus = 0xC0000BBB;
pub const PDH_INVALID_ARGUMENT: PDHStatus = 0xC0000BBD;

pub fn pdh_status_friendly_name(s: PDHStatus) -> String {
    match s {
        PDH_CSTATUS_NO_OBJECT => "PDH_CSTATUS_NO_OBJECT".to_owned(),
        PDH_CSTATUS_NO_MACHINE => "PDH_CSTATUS_NO_MACHINE".to_owned(),
        PDH_MORE_DATA => "PDH_MORE_DATA".to_owned(),
        PDH_MEMORY_ALLOCATION_FAILURE => "PDH_MEMORY_ALLOCATION_FAILURE".to_owned(),
        PDH_INVALID_ARGUMENT => "PDH_INVALID_ARGUMENT".to_owned(),
        _ => format!("{}", s),
    }
}
