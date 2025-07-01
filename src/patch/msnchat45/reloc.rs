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

use crate::patch::{utils::find_module_base};

/// The default filename for the MSN Chat Control (case-insensitive)
pub const DLL_NAME: &str = "MSNChat45.ocx";

/// The base address assumed during static analysis (e.g. IDA/Ghidra).
pub const PREFERRED_BASE_ADDRESS: usize = 0x37200000;

pub struct PatchContext {
    delta: usize,
}

impl PatchContext {
    /// Creates a new PatchContext by resolving the actual DLL base at runtime.
    pub fn new() -> Option<Self> {
        let actual = find_module_base(DLL_NAME)?;
        Some(Self {
            delta: actual.wrapping_sub(PREFERRED_BASE_ADDRESS),
        })
    }

    /// Adjusts a static address (e.g. from IDA) into a runtime-corrected address.
    pub fn adjust(&self, static_addr: usize) -> usize {
        static_addr.wrapping_add(self.delta)
    }
}
