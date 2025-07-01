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

pub fn apply_patches() {
    let dll_name = "MSNChat45.ocx";
    if let Some(ctx) = PatchContext::new() {
        #[cfg(debug_assertions)]
        println!(
            "Found module base for {}: {:#010x}",
            dll_name,
            ctx.adjust(0x37200000)
        );
        unsafe {
            // TODO: Add patches
            patch_bytes(ctx.adjust(0x3720006c), &[0x44, 0x4F, 0x54]); // Writing "DOS" where "DOS" was to suppress warnings. 
        }
    } else {
        #[cfg(debug_assertions)]
        eprintln!(
            "Failed to resolve module base for {} — cannot patch.",
            dll_name
        );
    }
}
