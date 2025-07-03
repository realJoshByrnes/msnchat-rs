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

pub unsafe fn init(ctx: &PatchContext) {
    unsafe { disable_oper_check(ctx); }
}

/// Disables the IRC operator check in the CTCP version response.
///
/// The CTCP version response will now always be sent, regardless of whether the
/// user is an IRC operator or not.
pub unsafe fn disable_oper_check(ctx: &PatchContext) {
    unsafe { patch_bytes(ctx.adjust(0x3722E83B), &[0x90, 0x90, 0x90, 0x90]) };
}