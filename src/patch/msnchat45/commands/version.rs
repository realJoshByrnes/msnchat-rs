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

use crate::patch::{
    msnchat45::reloc::PatchContext,
    utils::{encode_utf16z, patch_bytes},
};

/// Patch the /version command in MSN Chat 4.5 to display the current
/// executable's version number in the chat window. This is done by
/// patching in the command string and version number string into the
/// version command at the specified addresses.

pub unsafe fn init(ctx: &PatchContext) {
    unsafe {
        update_version_command_string(ctx.adjust(0x37218A07));
        update_version_command_version_string(ctx.adjust(0x37218A15));
    }
}

/// Patch the string used by the /version command in MSN Chat 4.5 to
/// include the current executable's name. This is done by patching in
/// the new string at the specified address.

unsafe fn update_version_command_string(addr: usize) {
    let package_name = env!("CARGO_PKG_NAME");
    let version_cmd_string = format!("{} v", package_name);
    let version_cmd_string_wide = encode_utf16z(&version_cmd_string);
    let version_cmd_string_wide = Box::leak(Box::new(version_cmd_string_wide));

    let version_cmd_string_ptr = version_cmd_string_wide.as_ptr() as usize;
    let version_cmd_string_bytes = version_cmd_string_ptr.to_le_bytes();

    #[cfg(debug_assertions)]
    println!(
        "Patching pointer at 0x{:08X} with wide string: {:?}",
        addr, version_cmd_string
    );
    unsafe { patch_bytes(addr, &version_cmd_string_bytes) };
}

/// Patch the /version command version string in MSN Chat 4.5 at the specified
/// address with the current executable's version number. The version number
/// is obtained from the `CARGO_PKG_VERSION` environment variable, and it is
/// formatted and encoded as a null-terminated UTF-8 string before being patched
/// into the specified address.

unsafe fn update_version_command_version_string(addr: usize) {
    let package_version = env!("CARGO_PKG_VERSION");
    let version_cmd_version_string = format!("{}\0", package_version);
    let version_cmd_version_string = Box::leak(Box::new(version_cmd_version_string));

    let version_cmd_version_string_ptr = version_cmd_version_string.as_ptr() as usize;
    let version_cmd_version_string_bytes = version_cmd_version_string_ptr.to_le_bytes();

    #[cfg(debug_assertions)]
    println!(
        "Patching pointer at 0x{:08X} with string: {:?}",
        addr, version_cmd_version_string
    );
    unsafe { patch_bytes(addr, &version_cmd_version_string_bytes) };
}
