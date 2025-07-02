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

use std::sync::OnceLock;

use crate::patch::utils::find_module_base;

/// The default filename for the MSN Chat Control (case-insensitive)
pub const DLL_NAME: &str = "MSNChat45.ocx";

/// The base address assumed during static analysis (e.g. IDA/Ghidra).
pub const PREFERRED_BASE_ADDRESS: usize = 0x37200000;

static PATCH_CONTEXT: OnceLock<PatchContext> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct PatchContext {
    delta: usize,
}

impl PatchContext {
    /// Retrieves a static reference to the `PatchContext`.
    ///
    /// This function ensures that the `PatchContext` is initialized only once
    /// using a `OnceLock`. If the initialization fails, it will panic with the
    /// message "PatchContext initialization failed". This context is used to
    /// adjust static addresses to runtime-corrected addresses.

    pub fn get() -> Option<&'static PatchContext> {
        if let Some(ctx) = PATCH_CONTEXT.get() {
            Some(ctx)
        } else {
            let ctx = PatchContext::new()?;
            PATCH_CONTEXT.set(ctx).ok()?; // handles race-condition safety
            PATCH_CONTEXT.get()
        }
    }

    /// Creates a new `PatchContext` by calculating the delta between the
    /// loaded module's base address and the preferred base address.
    ///
    /// This function attempts to find the base address of the MSN Chat Control
    /// using `find_module_base`. If successful, it calculates the difference
    /// (delta) between the actual base address and the preferred base address.
    /// This delta is used for adjusting static addresses to their correct
    /// runtime equivalents. Returns `Some(PatchContext)` if the base address
    /// is found, otherwise returns `None`.

    fn new() -> Option<Self> {
        let actual: usize = find_module_base(DLL_NAME)?;
        Some(Self {
            delta: actual.wrapping_sub(PREFERRED_BASE_ADDRESS),
        })
    }

    /// Adjusts a static address to the correct runtime address using the
    /// delta calculated when this `PatchContext` was created.
    ///
    /// This function takes a static address (i.e. an address found in the
    /// disassembly of the library) and adds the delta to it to obtain
    /// the correct runtime address. The delta is the difference between
    /// the preferred base address (used for static analysis) and the
    /// actual base address of the loaded module.

    pub fn adjust(&self, static_addr: usize) -> usize {
        static_addr.wrapping_add(self.delta)
    }
}
