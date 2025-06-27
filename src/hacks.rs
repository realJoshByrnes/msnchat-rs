use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::ProcessStatus::{GetModuleFileNameExW, K32EnumProcessModules};
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::{Foundation::HMODULE, System::Threading::PROCESS_ACCESS_RIGHTS};
use windows::Win32::System::Memory::{VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS};

const PROCESS_QUERY_INFORMATION: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(0x0400); // Standard access to query process info
const PROCESS_VM_READ: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(0x0010); // Required to read memory for module info

pub fn init_hacks() {
    // This function is for testing out what we can do to the MSN Chat Control whilst it's running.

    unsafe {
        let host_process_id = windows::Win32::System::Threading::GetCurrentProcessId();
        let activex_dll_name = OsStr::new("MsnChat45.ocx");

        let base = match get_module_base_address(host_process_id, activex_dll_name) {
            Some(base_address) => base_address,
            None => {
                println!(
                    "ActiveX control '{}' not found or unable to get its base address in process {}.",
                    activex_dll_name.to_string_lossy(),
                    host_process_id
                );
                return;
            }
        };

        let target_addr = 0x3722E83B as *mut u8;
        let patch_bytes = [0x90, 0x90, 0x90, 0x90]; // NOP the check that source was OPER

        // Disable memory protection
        let mut old_protect = PAGE_PROTECTION_FLAGS(0);
        let success = VirtualProtect(
            target_addr as *mut _,
            patch_bytes.len(),
            PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        );
        println!("Disabled memory protection: {:?}", success);
        
        // Write patch
        std::ptr::copy_nonoverlapping(patch_bytes.as_ptr(), target_addr as *mut u8, patch_bytes.len());

        // Restore memory protection
        let success = VirtualProtect(
            target_addr as *mut _,
            patch_bytes.len(),
            PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        );
        println!("Restored memory protection: {:?}", success);

        println!(
            "Base address of '{}' in process {} is: 0x{:X}",
            activex_dll_name.to_string_lossy(),
            host_process_id,
            base.0 as usize
        );
    }

    //0x71e36 = "Please wait..."
    // unsafe {
    //     // write non overlap
    //     let newstr = w!("Hello, world!");
    //     copy_nonoverlapping(newstr.as_ptr(), base.0 as *mut u16, 8);
    // }
}

fn get_module_base_address(process_id: u32, module_name: &OsStr) -> Option<HMODULE> {
    let process_handle = match unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            process_id,
        )
    } {
        Ok(val) => val,
        Err(err) => {
            eprintln!("Failed to open process {}. Error: {}", process_id, err);
            return None;
        }
    };

    let mut h_mods: [HMODULE; 1024] = [HMODULE(std::ptr::null_mut()); 1024];
    let mut cb_needed: u32 = 0;

    let result: bool = unsafe {
        K32EnumProcessModules(
            process_handle.clone(),
            h_mods.as_mut_ptr(),
            std::mem::size_of_val(&h_mods) as u32,
            &mut cb_needed,
        )
        .into()
    };

    if !result {
        eprintln!("Failed to enumerate process modules. Error: {}", unsafe {
            windows::Win32::Foundation::GetLastError().0
        });
        let _ = unsafe { CloseHandle(process_handle) };
        return None;
    }

    let num_modules = (cb_needed / std::mem::size_of::<HMODULE>() as u32) as usize;

    for i in 0..num_modules {
        let h_module = h_mods[i];
        let mut module_path_buffer = [0u16; 260]; // MAX_PATH wide chars

        let chars_copied = unsafe {
            GetModuleFileNameExW(
                Some(process_handle),
                Some(h_module),
                &mut module_path_buffer,
            )
        };

        if chars_copied > 0 {
            let path_os_string = OsString::from_wide(&module_path_buffer[..chars_copied as usize]);
            let path_buf = PathBuf::from(path_os_string);

            if let Some(filename) = path_buf.file_name() {
                if filename.eq_ignore_ascii_case(module_name) {
                    let _ = unsafe { CloseHandle(process_handle) };
                    return Some(h_module);
                }
            }
        }
    }

    let _ = unsafe { CloseHandle(process_handle) };
    None
}
