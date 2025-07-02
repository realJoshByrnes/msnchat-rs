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

use crate::patch::msnchat45::{commands, reloc::PatchContext};

/// Applies necessary patches to the MSN Chat Control at runtime.
///
/// This function attempts to locate the base address of the DLL and applies
/// specific byte patches to it. The `PatchContext` is used to resolve the
/// actual base address and adjust any static addresses accordingly. If the
/// module base cannot be resolved, the patching process is aborted. This
/// function is intended to modify behavior by altering the memory of the
/// loaded DLL.

pub fn apply_patches() {
    let dll_name = "MSNChat45.ocx";
    if let Some(ctx) = PatchContext::get() {
        #[cfg(debug_assertions)]
        println!(
            "Found module base for {}: {:#010x}",
            dll_name,
            ctx.adjust(0x37200000)
        );
        unsafe {
            commands::init(ctx.clone());
            commands::version::init(&ctx);
        }
    } else {
        #[cfg(debug_assertions)]
        eprintln!(
            "Failed to resolve module base for {} — cannot patch.",
            dll_name
        );
    }
}
