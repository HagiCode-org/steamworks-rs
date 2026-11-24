use super::*;

/// Access to the steam HTTP interface
pub struct Http {
    pub(crate) http: *mut sys::ISteamHTTP,
    pub(crate) inner: Arc<Inner>,
}
