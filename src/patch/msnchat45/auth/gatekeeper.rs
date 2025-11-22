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

use windows::{Win32::System::Com::CoCreateGuid, core::GUID};

use crate::patch::{msnchat45::reloc::PatchContext, utils::make_call_rel32};

pub fn init(ctx: &PatchContext) {
    make_call_rel32(ctx.adjust(0x3721A9BE), use_gate_keeper_id as usize);
}

// This function is called to store the GateKeeper ID in the appropriate places.
// If the provided GUID is zeroed, we generate a new one. This usually happens when the user hasn't registered the OCX.
// We then call the original function to store the GUID in the Channel Server and Directory Server structures
extern "thiscall" fn use_gate_keeper_id(this: *mut u8, gate_keeper_id: *mut GUID) {
    let ctx = PatchContext::get().unwrap();

    if unsafe { *gate_keeper_id } == GUID::zeroed() {
        #[cfg(debug_assertions)]
        eprintln!("Warning: GateKeeper ID was null!");
        match unsafe { CoCreateGuid() } {
            Ok(guid) => unsafe { *gate_keeper_id = guid },
            Err(e) => eprintln!("Failed to create GateKeeper ID: {:?}", e),
        }
    }
    #[cfg(debug_assertions)]
    println!("Using GateKeeper ID: {:?}", unsafe { *gate_keeper_id });

    unsafe {
        let sub_3723050_d: extern "thiscall" fn(_, _) = std::mem::transmute(ctx.adjust(0x3723050D));
        sub_3723050_d(this.add(7272), gate_keeper_id); // Channel Server
        sub_3723050_d(this.add(28), gate_keeper_id); // Directory Server
    }
}
