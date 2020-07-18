#![allow(dead_code)]

pub type PDHStatus = u32;

// pdh.h
pub const PDH_MAX_COUNTER_PATH: PDHStatus = 2048;
pub const PDH_MAX_COUNTER_NAME: PDHStatus = 1024;
pub const PDH_MAX_INSTANCE_NAME: PDHStatus = 1024;
pub const PDH_MAX_DATASOURCE_PATH: PDHStatus = 1024;
// pdhmsg.h
pub const PDH_DIALOG_CANCELLED: PDHStatus = 0x800007D9;
pub const PDH_MORE_DATA: PDHStatus = 0x800007D2;
pub const PDH_MEMORY_ALLOCATION_FAILURE: PDHStatus = 0xC0000BBB;
pub const PDH_INVALID_ARGUMENT: PDHStatus = 0xC0000BBD;
pub const PDH_INVALID_DATA: u32 = 0xC0000BC6;
pub const PDH_INVALID_HANDLE: u32 = 0xC0000BBC;
pub const PDH_CSTATUS_NO_OBJECT: PDHStatus = 0xC0000BB8;
pub const PDH_CSTATUS_NO_MACHINE: PDHStatus = 0x800007D0;
pub const PDH_CSTATUS_NO_INSTANCE: u32 = 0x800007D1;
pub const PDH_CSTATUS_NO_COUNTER: u32 = 0xC0000BB9;
pub const PDH_CSTATUS_BAD_COUNTERNAME: u32 = 0xC0000BC0;

pub fn pdh_status_friendly_name(s: PDHStatus) -> String {
    match s {
        PDH_CSTATUS_NO_OBJECT => "PDH_CSTATUS_NO_OBJECT".to_owned(),
        PDH_CSTATUS_NO_MACHINE => "PDH_CSTATUS_NO_MACHINE".to_owned(),
        PDH_MORE_DATA => "PDH_MORE_DATA".to_owned(),
        PDH_MEMORY_ALLOCATION_FAILURE => "PDH_MEMORY_ALLOCATION_FAILURE".to_owned(),
        PDH_INVALID_ARGUMENT => "PDH_INVALID_ARGUMENT".to_owned(),
        PDH_INVALID_DATA => "PDH_INVALID_DATA".to_owned(),
        PDH_INVALID_HANDLE => "PDH_INVALID_HANDLE".to_owned(),
        PDH_CSTATUS_NO_INSTANCE => "PDH_CSTATUS_NO_INSTANCE".to_owned(),
        PDH_CSTATUS_NO_COUNTER => "PDH_CSTATUS_NO_COUNTER".to_owned(),
        PDH_CSTATUS_BAD_COUNTERNAME => "PDH_CSTATUS_BAD_COUNTERNAME".to_owned(),
        _ => format!("{}", s),
    }
}

// PDH formatting constants
/// Format the pdh counter as a f64
pub const PDH_FMT_DOUBLE: u32 = 0x00000200;
/// Format the pdh counter as an i32
pub const PDH_FMT_LONG: u32 = 0x00000100;
/// Format the pdh counter as an i64
pub const PDH_FMT_LARGE: u32 = 0x00000400;
pub const PDH_FMT_RAW: u32 = 0x00000010;
pub const PDH_FMT_ANSI: u32 = 0x00000020;
pub const PDH_FMT_UNICODE: u32 = 0x00000040;
