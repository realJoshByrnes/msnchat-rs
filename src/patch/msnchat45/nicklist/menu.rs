// msnchat-rs
// Copyright (C) 2025 Joshua Byrnes
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::patch::{msnchat45::reloc::PatchContext, utils::patch_bytes};

pub fn init(ctx: &PatchContext) {
    let addr = 0x37201E5C;
    let dst = nicklist_menu_wrapper as usize;
    #[cfg(debug_assertions)]
    println!("Patching off_{:08X} with 0x{:08X}", addr, dst);
    let rel = dst.to_le_bytes();
    let bytes = [rel[0], rel[1], rel[2], rel[3]];
    unsafe { patch_bytes(ctx.adjust(addr), &bytes) };
}

type DoUserActionFn = extern "stdcall" fn(i32, i32, i32, i32) -> i32;
type GetSelectedItemsFn = extern "stdcall" fn(i32, i32, *mut i32, *mut i32) -> i32;
type SetChannelUserModeFn = unsafe extern "thiscall" fn(this: i32, a2: i32, a3: i32, a4: i32) -> i8;
const NAME_OFFSET: usize = 12;

#[unsafe(no_mangle)]
pub extern "stdcall" fn nicklist_menu_wrapper(a1: i32, a2: i32, a3: i32, a4: i32) -> i32 {
    let ctx = PatchContext::get().unwrap();
    if a3 <= 13011 {
        // Let the default handler do it's job
        let do_user_action: DoUserActionFn = unsafe { std::mem::transmute(ctx.adjust(0x3721CE64)) };
        return do_user_action(a1, a2, a3, a4);
    }
    unsafe {
        let mut v22: i32 = 0;
        let mut v23: i32 = 0;

        let v5_addr = (a1 as usize).wrapping_add(16904);

        let v6_ptr = v5_addr as *const usize;
        let v6 = *v6_ptr;

        let get_selected_fn_ptr_addr = v6.wrapping_add(56);
        let get_selected_fn_ptr = get_selected_fn_ptr_addr as *const GetSelectedItemsFn;

        let result = (*get_selected_fn_ptr)(v5_addr as i32, -1, &mut v22, &mut v23);

        if result >= 0 && v23 > 0 {
            for i in 0..v23 {
                let v13_ptr_addr = (v22 as usize).wrapping_add((4 * i) as usize);
                let v13: usize = *(v13_ptr_addr as *const usize);

                let name_ptr_addr = (v13 as *const u8).add(NAME_OFFSET) as *const *const u8;
                let name_ptr = *name_ptr_addr;

                if !name_ptr.is_null() {
                    let mut name_len = 0;
                    while *name_ptr.add(name_len) != 0 {
                        name_len += 1;
                    }
                    let name_slice = std::slice::from_raw_parts(name_ptr, name_len);
                    // Warning: Names may not be valid UTF8 and should only be used for display (eg. Logging)
                    let name = String::from_utf8_lossy(name_slice);

                    #[cfg(debug_assertions)]
                    println!("Menu item {} selected for user {}", a3, name);

                    match a3 {
                        13012 => {
                            // Owner (+q)
                            #[cfg(debug_assertions)]
                            println!("Attempting to +q {}", name);
                            let set_channel_user_mode: SetChannelUserModeFn =
                                std::mem::transmute(ctx.adjust(0x3722CEDF) as usize);
                            set_channel_user_mode(a1 + 44, v13 as i32, 8, 1);
                        }
                        // We can add new functions here
                        _ => {}
                    }
                } else {
                    #[cfg(debug_assertions)]
                    println!(
                        "Menu item {} was selected, but the username pointer was a null pointer.",
                        a3
                    );
                }
            }
        }
        0 // The default handler always seems to return 0
    }
}
