use winapi::shared::ntdef::NULL;
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::pdh::{
    PdhBrowseCountersW, PdhCloseQuery, PdhOpenQueryW, PDH_BROWSE_DLG_CONFIG_W, PDH_HQUERY as HQuery,
};

use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;

mod pdhmsg;

pub fn browse_perf_counters() {
    unsafe {
        let mut query: HQuery = NULL;
        let mut status = PdhOpenQueryW(&0, 0, &mut query) as u32;
        // TODO Is this even correct? I feel like I shouldn't have to cast this.
        if status != ERROR_SUCCESS {
            // we should go to the cleanup section now.
            println!("PdhOpenQueryA failed with status {}", status);
        } else {
            let mut msg: Vec<u16> = OsStr::new("Select a counter to monitor.")
                .encode_wide()
                .chain(once(0))
                .collect();
            let mut return_path_buf = Vec::<u16>::with_capacity(1024);
            let mut browse_dlg = PDH_BROWSE_DLG_CONFIG_W {
                flags: 0,
                CallBackStatus: ERROR_SUCCESS as i32,
                hWndOwner: null_mut(),
                szDataSource: null_mut(),                         // TODO
                szReturnPathBuffer: return_path_buf.as_mut_ptr(), // TODO
                cchReturnPathLength: 1024,                        // TODO
                pCallBack: None,                                  // TODO
                dwCallBackArg: 0,                                 // TODO
                dwDefaultDetailLevel: 0,                          // TODO
                szDialogBoxCaption: msg.as_mut_ptr(),             // TODO
            };
            status = PdhBrowseCountersW(&mut browse_dlg) as u32;
            if status != ERROR_SUCCESS {
                if status == pdhmsg::PDH_DIALOG_CANCELLED {
                    println!("Dialog canceled");
                } else {
                    println!("PdhBrowseCountersW failed with status {}", status);
                }
            } else if return_path_buf.len() == 0 {
                println!("User didn't select counter");
            } else {
                println!(
                    "Counter selected {}",
                    String::from_utf16_lossy(return_path_buf.as_ref())
                );
            }
        }
        // Cleanup code this should go in a drop probably
        if query != NULL {
            PdhCloseQuery(query);
        }
    }
}
