pub struct ModuleInfo {
    base_address: usize,
}

impl ModuleInfo {
    pub fn new(base_address: usize) -> Self {
        Self { base_address }
    }

    pub fn resolve(&self, address: usize) -> *mut std::ffi::c_void {
        // If the provided address is an absolute address (e.g. 0x3721da6c or 0x7321da6c)
        // calculate the RVA by subtracting the preferred base address.
        // Based on the user's snippet, we assume the preferred base address is 0x37200000,
        // or 0x73200000. Let's handle both.
        let rva = if (0x73200000..0x73300000).contains(&address) {
            address - 0x73200000
        } else if (0x37200000..0x37300000).contains(&address) {
            address - 0x37200000
        } else {
            address
        };
        (self.base_address + rva) as *mut std::ffi::c_void
    }
}
