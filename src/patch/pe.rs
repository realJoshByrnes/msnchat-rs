use pelite::pe32::Pe;
use std::ffi::c_void;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
    PAGE_PROTECTION_FLAGS, PAGE_READONLY, PAGE_READWRITE, VirtualAlloc, VirtualProtect,
};
use windows::core::PCSTR;

pub struct ManualModule {
    pub base_address: *mut u8,
}

impl ManualModule {
    /// # Safety
    /// This function loads and executes native code from memory, which inherently bypasses
    /// the OS loader and can cause undefined behavior if the PE image is malformed or changes
    /// memory protections.
    pub unsafe fn load(bytes: &[u8]) -> Result<Self, String> {
        unsafe {
            log::info!("ManualModule::load started");
            // 1. Parsing headers
            let pe = pelite::pe32::PeFile::from_bytes(bytes).map_err(|e| e.to_string())?;
            log::info!("Parsed PE headers successfully");
            let optional_header = pe.optional_header();
            let image_size = optional_header.SizeOfImage as usize;
            let preferred_base = optional_header.ImageBase as usize;

            // 1. Allocate Memory
            let mut base_address = VirtualAlloc(
                Some(preferred_base as *mut c_void),
                image_size,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            );

            // If preferred base is taken, let OS assign one
            if base_address.is_null() {
                base_address =
                    VirtualAlloc(None, image_size, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
                if base_address.is_null() {
                    return Err("Failed to allocate memory for PE image".to_string());
                }
            }

            let base_ptr = base_address as *mut u8;
            log::info!("Allocated memory at {:p}", base_ptr);

            // 2. Copy Headers
            let header_size = optional_header.SizeOfHeaders as usize;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), base_ptr, header_size);
            log::info!("Copied headers");

            // 3. Map Sections
            for section in pe.section_headers() {
                if section.VirtualSize == 0 {
                    continue;
                }

                let dest = base_ptr.add(section.VirtualAddress as usize);
                let src = bytes.as_ptr().add(section.PointerToRawData as usize);
                let size = section.SizeOfRawData as usize;

                if size > 0 {
                    std::ptr::copy_nonoverlapping(src, dest, size);
                }
            }
            log::info!("Loaded sections successfully");

            // 4. Apply Base Relocations
            let delta = (base_ptr as isize) - (preferred_base as isize);
            let data_dirs = optional_header.DataDirectory.as_ptr();
            let reloc_dir = *data_dirs.add(pelite::image::IMAGE_DIRECTORY_ENTRY_BASERELOC);
            if reloc_dir.VirtualAddress != 0 && delta != 0 {
                let mut reloc_ptr = base_ptr.add(reloc_dir.VirtualAddress as usize);
                let reloc_end = reloc_ptr.add(reloc_dir.Size as usize);
                while reloc_ptr < reloc_end {
                    let page_rva = *(reloc_ptr as *const u32);
                    let block_size = *(reloc_ptr.add(4) as *const u32);
                    if block_size == 0 {
                        break;
                    } // safety against malformed blocks
                    let word_ptr = reloc_ptr.add(8) as *const u16;
                    let num_words = (block_size - 8) / 2;
                    for i in 0..num_words {
                        let word = *word_ptr.add(i as usize);
                        let ty = word >> 12;
                        let offset = word & 0xFFF;
                        if ty == pelite::image::IMAGE_REL_BASED_HIGHLOW as u16 {
                            let patch_addr =
                                base_ptr.add(page_rva as usize + offset as usize) as *mut u32;
                            *patch_addr = (*patch_addr).wrapping_add(delta as u32);
                        }
                    }
                    reloc_ptr = reloc_ptr.add(block_size as usize);
                }
            }
            log::info!("Applied base relocations successfully");

            // 5. Parse Imports
            if let Ok(imports) = pe.imports() {
                for desc in imports {
                    if let Ok(dll_name) = desc.dll_name() {
                        let dll_str = dll_name.to_str().unwrap_or("");
                        let dll_c = std::ffi::CString::new(dll_str).unwrap();
                        let hmod = LoadLibraryA(PCSTR::from_raw(dll_c.as_ptr() as *const u8))
                            .map_err(|e| format!("LoadLibraryA failed for {:?}: {}", dll_str, e))?;

                        let iat = base_ptr.add(desc.image().FirstThunk as usize) as *mut usize;
                        let int = if desc.image().OriginalFirstThunk != 0 {
                            base_ptr.add(desc.image().OriginalFirstThunk as usize) as *const usize
                        } else {
                            iat as *const usize
                        };

                        let mut i = 0;
                        while *int.add(i) != 0 {
                            let thunk = *int.add(i);
                            let is_ordinal = (thunk & 0x80000000) != 0;

                            let proc_addr = if is_ordinal {
                                let ordinal = thunk & 0xFFFF;
                                GetProcAddress(hmod, PCSTR::from_raw(ordinal as *const u8))
                            } else {
                                let name_rva = (thunk & 0x7FFFFFFF) as usize;
                                let name_ptr = base_ptr.add(name_rva + 2);
                                GetProcAddress(hmod, PCSTR::from_raw(name_ptr))
                            };

                            if let Some(addr) = proc_addr {
                                *iat.add(i) = addr as usize;
                            } else {
                                return Err(format!("Failed to resolve import in {}", dll_str));
                            }
                            i += 1;
                        }
                    }
                }
            }
            log::info!("Parsed imports successfully");

            // 6. Apply Memory Protections
            for section in pe.section_headers() {
                if section.VirtualSize == 0 {
                    continue;
                }

                let dest = base_ptr.add(section.VirtualAddress as usize);
                let size = section.VirtualSize as usize;
                let chars = section.Characteristics;

                let exec = (chars & pelite::image::IMAGE_SCN_MEM_EXECUTE) != 0;
                let read = (chars & pelite::image::IMAGE_SCN_MEM_READ) != 0;
                let write = (chars & pelite::image::IMAGE_SCN_MEM_WRITE) != 0;

                let protect = if exec && read && write {
                    PAGE_EXECUTE_READWRITE
                } else if exec && read {
                    PAGE_EXECUTE_READ
                } else if exec {
                    PAGE_EXECUTE
                } else if read && write {
                    PAGE_READWRITE
                } else if read {
                    PAGE_READONLY
                } else {
                    PAGE_PROTECTION_FLAGS(0) // Optional: no access
                };

                if protect.0 != 0 {
                    let mut old_protect = PAGE_PROTECTION_FLAGS::default();
                    VirtualProtect(dest as *const c_void, size, protect, &mut old_protect)
                        .map_err(|e| format!("VirtualProtect failed: {}", e))?;
                }
            }
            log::info!("Applied memory protections successfully");

            // 7. Execute DllMainTLS Callbacks (Skipping for now as OCX typically doesn't use complex TLS)

            // Apply Patches that would normally be triggered by LoadLibrary Hook (Must be done before DllMain)
            log::info!("Applying patches to manually loaded DLL...");
            let module_info = crate::patch::module_info::ModuleInfo::new(base_ptr as usize);
            if let Err(e) = crate::patch::gatekeeper::apply(&module_info) {
                log::error!("Failed to apply gatekeeper patch: {}", e);
            }
            if let Err(e) = crate::patch::atl_thunk::apply(&module_info) {
                log::error!("Failed to apply atl thunk patch: {}", e);
            }

            // Apply queued hooks
            if let Err(status) = minhook::MinHook::apply_queued() {
                log::error!("Failed to apply queued hooks: {:?}", status);
            }

            // 8. Execute DllMain
            let entry_point_rva = optional_header.AddressOfEntryPoint as usize;
            if entry_point_rva != 0 {
                let entry_point = base_ptr.add(entry_point_rva);
                let dll_main: extern "system" fn(
                    HINSTANCE,
                    u32,
                    *const c_void,
                ) -> windows::core::BOOL = std::mem::transmute(entry_point);

                let result = dll_main(
                    HINSTANCE(base_ptr as *mut c_void),
                    1, // DLL_PROCESS_ATTACH
                    std::ptr::null_mut(),
                );

                let success = result.0;
                log::info!("DllMain executed. Success: {}", success);
                if success == 0 {
                    return Err("DllMain returned FALSE".into());
                }
            }

            log::info!("Manual PE mapping and patching complete.");

            Ok(Self {
                base_address: base_ptr,
            })
        }
    }

    /// # Safety
    /// Bypasses type checking when obtaining exported function pointers.
    pub unsafe fn get_export(&self, name: &str) -> Result<*mut c_void, String> {
        unsafe {
            let base_ptr = self.base_address;
            let dos_header = base_ptr as *const pelite::image::IMAGE_DOS_HEADER;
            if (*dos_header).e_magic != pelite::image::IMAGE_DOS_SIGNATURE {
                return Err("Invalid DOS signature".into());
            }

            let nt_headers = base_ptr.add((*dos_header).e_lfanew as usize)
                as *const pelite::image::IMAGE_NT_HEADERS32;
            let data_dirs = (*nt_headers).OptionalHeader.DataDirectory.as_ptr();
            let export_dir_rva =
                (*data_dirs.add(pelite::image::IMAGE_DIRECTORY_ENTRY_EXPORT)).VirtualAddress;
            if export_dir_rva == 0 {
                return Err("No exports found".into());
            }

            let export_dir = base_ptr.add(export_dir_rva as usize)
                as *const pelite::image::IMAGE_EXPORT_DIRECTORY;
            let names_ptr = base_ptr.add((*export_dir).AddressOfNames as usize) as *const u32;
            let funcs_ptr = base_ptr.add((*export_dir).AddressOfFunctions as usize) as *const u32;
            let ords_ptr = base_ptr.add((*export_dir).AddressOfNameOrdinals as usize) as *const u16;

            for i in 0..(*export_dir).NumberOfNames {
                let name_rva = *names_ptr.add(i as usize);
                let name_str_ptr = base_ptr.add(name_rva as usize) as *const std::ffi::c_char;
                let current_name = std::ffi::CStr::from_ptr(name_str_ptr)
                    .to_str()
                    .unwrap_or("");
                if current_name == name {
                    let ordinal = *ords_ptr.add(i as usize);
                    let func_rva = *funcs_ptr.add(ordinal as usize);
                    return Ok(base_ptr.add(func_rva as usize) as *mut c_void);
                }
            }

            Err(format!("Export '{}' not found", name))
        }
    }
}
