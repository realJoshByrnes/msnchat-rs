#[derive(Clone, Copy)]
pub struct ModuleInfo {
    pub base_address: usize,
}

impl ModuleInfo {
    pub fn new(base_address: usize) -> Self {
        Self { base_address }
    }

    pub fn resolve(&self, rva: usize) -> *mut std::ffi::c_void {
        // MSNChat45.ocx default image base is 0x37200000
        let offset = rva - 0x37200000;
        (self.base_address + offset) as *mut std::ffi::c_void
    }
}
