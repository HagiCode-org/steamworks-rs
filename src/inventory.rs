use super::*;

/// Access to the steam inventory interface
pub struct Inventory {
    pub(crate) inventory: *mut sys::ISteamInventory,
    pub(crate) inner: Arc<Inner>,
}
