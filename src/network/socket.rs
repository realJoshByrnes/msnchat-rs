use std::ffi::c_void;
use tokio::sync::mpsc::UnboundedSender;

pub struct RustSocket {
    pub id: u32,
    pub tx: Option<UnboundedSender<Vec<u8>>>,
    pub rx_buffer: Vec<u8>,
    pub callback_ptr: *mut c_void,
    pub context_ptr: *mut c_void,
    pub connected: bool,
    pub closed: bool,
}

unsafe impl Send for RustSocket {}
unsafe impl Sync for RustSocket {}

impl RustSocket {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            tx: None,
            rx_buffer: Vec::new(),
            callback_ptr: std::ptr::null_mut(),
            context_ptr: std::ptr::null_mut(),
            connected: false,
            closed: false,
        }
    }
}
