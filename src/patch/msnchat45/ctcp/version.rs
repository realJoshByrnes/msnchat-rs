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

// Hijack the string "IRC8\0" from the Chat Control's version reply. lstrlenA() will be used for an alloca() call.
const PATCHED_CTCP_VERSION_INCLUDING_IRC_VERSION: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " v",
    env!("CARGO_PKG_VERSION"),
    " - IRC8\0"
);

pub unsafe fn init(ctx: &PatchContext) {
    unsafe {
        disable_oper_check(ctx);
        patch_version_reply(ctx);
    }
}

/// Disables the IRC operator check in the CTCP version response.
///
/// The CTCP version response will now always be sent, regardless of whether the
/// user is an IRC operator or not.
pub unsafe fn disable_oper_check(ctx: &PatchContext) {
    unsafe { patch_bytes(ctx.adjust(0x3722E83B), &[0x90, 0x90, 0x90, 0x90]) };
}

pub unsafe fn patch_version_reply(ctx: &PatchContext) {
    let addr = ctx.adjust(0x3722E85B);
    let dst = PATCHED_CTCP_VERSION_INCLUDING_IRC_VERSION.as_ptr() as usize;
    #[cfg(debug_assertions)]
    println!("Patching 0x{:08X} with MOV EDI, imm32 to 0x{:08X}", addr, dst);
    let rel = dst.to_le_bytes();
    let bytes = [0xBF, rel[0], rel[1], rel[2], rel[3]];
    unsafe { patch_bytes(addr, &bytes) };
}
