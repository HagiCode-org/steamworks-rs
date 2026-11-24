use super::*;

use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::ptr;

/// Access to the Steam inventory interface (`ISteamInventory`).
///
/// Most methods start an asynchronous query. They return an [`InventoryResult`] that is still pending.
/// Steam reports that it is ready through [`InventoryResultReady`] while `run_callbacks()` runs, and
/// the result is then read with its accessors. Methods that change the user's real inventory say so
/// in their documentation.
pub struct Inventory {
    pub(crate) inventory: *mut sys::ISteamInventory,
    pub(crate) inner: Arc<Inner>,
}

/// The item instance id Steam uses for "no item" (`k_SteamItemInstanceIDInvalid`).
///
/// Pass it as the destination of [`Inventory::transfer_item_quantity()`] to split a stack into a new item.
pub const ITEM_INSTANCE_ID_INVALID: u64 = u64::MAX;

/// Item status flag: the item is account-locked and cannot be traded or given away (`k_ESteamItemNoTrade`).
pub const ITEM_FLAG_NO_TRADE: u16 = 1 << 0;
/// Action flag: the item was destroyed, traded away, expired or otherwise invalidated (`k_ESteamItemRemoved`).
pub const ITEM_FLAG_REMOVED: u16 = 1 << 8;
/// Action flag: the quantity was decreased by one through `ConsumeItem` (`k_ESteamItemConsumed`).
pub const ITEM_FLAG_CONSUMED: u16 = 1 << 9;

/// `k_SteamInventoryResultInvalid`.
const RESULT_INVALID: sys::SteamInventoryResult_t = -1;
/// `k_SteamInventoryUpdateHandleInvalid`.
const UPDATE_HANDLE_INVALID: sys::SteamInventoryUpdateHandle_t = u64::MAX;

/// One item instance in an inventory result (`SteamItemDetails_t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ItemDetails {
    /// The globally unique id of this item instance. It is never reused for another item.
    pub item_id: u64,
    /// The item definition id (`SteamItemDef_t`), a number between 1 and 999999999.
    pub definition: i32,
    /// The stack size of the item.
    pub quantity: u16,
    /// Status and action flags: see [`ITEM_FLAG_NO_TRADE`], [`ITEM_FLAG_REMOVED`] and [`ITEM_FLAG_CONSUMED`].
    pub flags: u16,
}

impl From<sys::SteamItemDetails_t> for ItemDetails {
    fn from(raw: sys::SteamItemDetails_t) -> Self {
        Self {
            item_id: raw.m_itemId,
            definition: raw.m_iDefinition,
            quantity: raw.m_unQuantity,
            flags: raw.m_unFlags,
        }
    }
}

/// The state of an [`InventoryResult`], as `GetResultStatus` reports it.
#[derive(Clone, Debug)]
pub enum InventoryStatus {
    /// Steam has not finished the query yet. Wait for its [`InventoryResultReady`] callback.
    Pending,
    /// The result is ready and complete.
    Ok,
    /// The result is ready but may be out of date, for example after one of its items was traded or
    /// consumed. Its items can still be read. See `DeserializeResult` in the SDK documentation.
    Expired,
    /// The query failed with Steam's error. The result has no usable items.
    Failed(SteamError),
}

/// A result set of an inventory query (`SteamInventoryResult_t`).
///
/// The handle is destroyed with `DestroyResult` when this value is dropped. Call [`leak()`](Self::leak)
/// to keep the handle alive on purpose, and destroy it yourself later.
pub struct InventoryResult {
    inventory: *mut sys::ISteamInventory,
    handle: sys::SteamInventoryResult_t,
}

// SAFETY: the handle is a plain integer and the interface pointer refers to Steam's process-wide
// inventory interface. The type is move-only, so one result is never used from two threads at once.
unsafe impl Send for InventoryResult {}

impl InventoryResult {
    /// Returns the raw `SteamInventoryResult_t`. Use it to match the result with
    /// [`InventoryResultReady::handle`].
    pub fn raw(&self) -> sys::SteamInventoryResult_t {
        self.handle
    }

    /// Gives up the handle without destroying it. The caller must call `DestroyResult` with the
    /// returned value once it is done, or the handle stays alive until Steam shuts down.
    pub fn leak(self) -> sys::SteamInventoryResult_t {
        let handle = self.handle;
        std::mem::forget(self);
        handle
    }

    /// Returns the state of the result. It stays [`InventoryStatus::Pending`] until Steam has finished
    /// the query, which is announced by [`InventoryResultReady`].
    pub fn status(&self) -> InventoryStatus {
        let result =
            unsafe { sys::SteamAPI_ISteamInventory_GetResultStatus(self.inventory, self.handle) };
        match result {
            sys::EResult::k_EResultOK => InventoryStatus::Ok,
            sys::EResult::k_EResultExpired => InventoryStatus::Expired,
            sys::EResult::k_EResultPending => InventoryStatus::Pending,
            error => InventoryStatus::Failed(error.into()),
        }
    }

    /// Copies the items of the result. Returns `None` when Steam refuses the read, for example while
    /// the result is still pending.
    ///
    /// The SDK header does not document how to get the count: this reads it with a first call that
    /// passes a NULL array, the same way `GetItemDefinitionIDs` is documented to work. Unverified
    /// against a live Steam client.
    pub fn items(&self) -> Option<Vec<ItemDetails>> {
        unsafe {
            let mut count = 0u32;
            if !sys::SteamAPI_ISteamInventory_GetResultItems(
                self.inventory,
                self.handle,
                ptr::null_mut(),
                &mut count,
            ) {
                return None;
            }
            let mut raw: Vec<sys::SteamItemDetails_t> =
                (0..count).map(|_| std::mem::zeroed()).collect();
            let mut written = count;
            if !sys::SteamAPI_ISteamInventory_GetResultItems(
                self.inventory,
                self.handle,
                raw.as_mut_ptr(),
                &mut written,
            ) {
                return None;
            }
            raw.truncate(written.min(count) as usize);
            Some(raw.into_iter().map(ItemDetails::from).collect())
        }
    }

    /// Reads a dynamic string property of the item at `index` in [`items()`](Self::items).
    ///
    /// Property names are ASCII letters, digits and underscores. Returns `None` when the name contains
    /// a NUL byte, when the index or the name is unknown, or when Steam refuses the read.
    pub fn item_property(&self, index: u32, name: &str) -> Option<String> {
        let name = CString::new(name).ok()?;
        read_string(|buf, size| unsafe {
            sys::SteamAPI_ISteamInventory_GetResultItemProperty(
                self.inventory,
                self.handle,
                index,
                name.as_ptr(),
                buf,
                size,
            )
        })
    }

    /// Returns the server time at which Steam generated the result. Compare it with the server time
    /// from `ISteamUtils::GetServerRealTime` to judge how old the result is. The unit is the one that
    /// function uses (Unix seconds, unverified against a live Steam client).
    pub fn timestamp(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamInventory_GetResultTimestamp(self.inventory, self.handle) }
    }

    /// Returns `true` when the result belongs to `id`. Check this after [`Inventory::deserialize_result()`],
    /// so that a remote player cannot pass off another player's inventory as their own.
    pub fn check_steam_id(&self, id: SteamId) -> bool {
        unsafe {
            sys::SteamAPI_ISteamInventory_CheckResultSteamID(self.inventory, self.handle, id.0)
        }
    }

    /// Serializes the result with its signature bytes, so that another player can check it with
    /// [`Inventory::deserialize_result()`]. The SDK recommends building a small result first with
    /// [`Inventory::get_items_by_id()`].
    ///
    /// Returns `None` when Steam refuses the call, for example while the result is still pending.
    /// Steam marks a serialized result as expired after one hour (see the SDK documentation of `DeserializeResult`). The items can still be read.
    pub fn serialize(&self) -> Option<Vec<u8>> {
        unsafe {
            let mut size = 0u32;
            if !sys::SteamAPI_ISteamInventory_SerializeResult(
                self.inventory,
                self.handle,
                ptr::null_mut(),
                &mut size,
            ) {
                return None;
            }
            let mut buf = vec![0u8; size as usize];
            let mut written = size;
            if !sys::SteamAPI_ISteamInventory_SerializeResult(
                self.inventory,
                self.handle,
                buf.as_mut_ptr() as *mut c_void,
                &mut written,
            ) {
                return None;
            }
            buf.truncate(written.min(size) as usize);
            Some(buf)
        }
    }
}

impl Drop for InventoryResult {
    fn drop(&mut self) {
        // SAFETY: the handle was created by this interface and is destroyed only here.
        unsafe { sys::SteamAPI_ISteamInventory_DestroyResult(self.inventory, self.handle) }
    }
}

/// A pending batch of property changes on inventory items (`SteamInventoryUpdateHandle_t`).
///
/// Build the batch with the setters, then call [`submit()`](Self::submit). The SDK has no call that
/// releases an unsubmitted batch, so dropping one without submitting it sends nothing.
pub struct PropertyUpdate {
    inventory: *mut sys::ISteamInventory,
    handle: sys::SteamInventoryUpdateHandle_t,
}

// SAFETY: the handle is a plain integer and the interface pointer is Steam's process-wide inventory
// interface. The batch is moved into `submit()`, so it is never used from two threads at once.
unsafe impl Send for PropertyUpdate {}

impl PropertyUpdate {
    /// Removes a dynamic property from an item in this batch. Returns `Err` when Steam refuses it,
    /// for example for an unknown item, or when the name contains a NUL byte.
    pub fn remove_property(&self, item_id: u64, name: &str) -> Result<(), ()> {
        let name = CString::new(name).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_RemoveProperty(
                self.inventory,
                self.handle,
                item_id,
                name.as_ptr(),
            )
        };
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Sets a string property on an item in this batch. Returns `Err` when Steam refuses it, or when
    /// the name or the value contains a NUL byte.
    pub fn set_string(&self, item_id: u64, name: &str, value: &str) -> Result<(), ()> {
        let name = CString::new(name).map_err(|_| ())?;
        let value = CString::new(value).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_SetPropertyString(
                self.inventory,
                self.handle,
                item_id,
                name.as_ptr(),
                value.as_ptr(),
            )
        };
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Sets a boolean property on an item in this batch. Returns `Err` when Steam refuses it, or when
    /// the name contains a NUL byte.
    pub fn set_bool(&self, item_id: u64, name: &str, value: bool) -> Result<(), ()> {
        let name = CString::new(name).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_SetPropertyBool(
                self.inventory,
                self.handle,
                item_id,
                name.as_ptr(),
                value,
            )
        };
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Sets a 64-bit integer property on an item in this batch. Returns `Err` when Steam refuses it,
    /// or when the name contains a NUL byte.
    pub fn set_i64(&self, item_id: u64, name: &str, value: i64) -> Result<(), ()> {
        let name = CString::new(name).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_SetPropertyInt64(
                self.inventory,
                self.handle,
                item_id,
                name.as_ptr(),
                value,
            )
        };
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Sets a float property on an item in this batch. Returns `Err` when Steam refuses it, or when
    /// the name contains a NUL byte.
    pub fn set_f32(&self, item_id: u64, name: &str, value: f32) -> Result<(), ()> {
        let name = CString::new(name).map_err(|_| ())?;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_SetPropertyFloat(
                self.inventory,
                self.handle,
                item_id,
                name.as_ptr(),
                value,
            )
        };
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Sends the batch to Steam. This changes the item properties of the user's real inventory.
    ///
    /// The batch is consumed. The returned result is pending until its [`InventoryResultReady`]
    /// callback arrives. Returns `None` when Steam refuses the batch.
    pub fn submit(self) -> Option<InventoryResult> {
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_SubmitUpdateProperties(
                self.inventory,
                self.handle,
                &mut handle,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }
}

/// The price of one item definition, in the currency reported by [`Inventory::request_prices()`].
///
/// The prices are in the currency's smallest unit as Steam reports them (unverified against a live
/// Steam client).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ItemPrice {
    /// The item definition id.
    pub definition: i32,
    /// The current price, after any sale.
    pub current: u64,
    /// The base price, without any sale.
    pub base: u64,
}

/// Callback `SteamInventoryResultReady_t`: an inventory result handle became ready, or failed.
///
/// Steam sends exactly one of these per handle, while `run_callbacks()` runs. A handle whose query
/// failed carries the error in `error`. An expired result is not an error: its items can still be
/// read, and [`InventoryResult::status()`] reports `Expired`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InventoryResultReady {
    /// The raw handle, to match with [`InventoryResult::raw()`].
    pub handle: sys::SteamInventoryResult_t,
    /// The query error, or `None` when the result is ready.
    pub error: Option<SteamError>,
}

impl_callback!(cb: SteamInventoryResultReady_t => InventoryResultReady {
    Self {
        handle: cb.m_handle,
        error: ready_error(cb.m_result),
    }
});

/// Callback `SteamInventoryFullUpdate_t`: a [`Inventory::get_all_items()`] result is newer than the
/// last known inventory. It arrives in addition to [`InventoryResultReady`] for the same handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InventoryFullUpdate {
    /// The raw handle of the new inventory result.
    pub handle: sys::SteamInventoryResult_t,
}

impl_callback!(cb: SteamInventoryFullUpdate_t => InventoryFullUpdate {
    Self { handle: cb.m_handle }
});

/// Callback `SteamInventoryDefinitionUpdate_t`: item definitions were loaded or refreshed, for
/// example after [`Inventory::load_item_definitions()`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InventoryDefinitionUpdate;

impl_callback!(_cb: SteamInventoryDefinitionUpdate_t => InventoryDefinitionUpdate {
    Self
});

/// Decodes a call result of the inventory interface and reports Steam's error code in `error`.
trait DecodeCallResult<S>: Sized {
    fn decode(raw: &S) -> Self;
    fn error(&self) -> Option<SteamError>;
}

/// The result of [`Inventory::request_eligible_promo_item_definitions()`].
///
/// `count` is the number of eligible item definitions. Read their ids with
/// [`Inventory::eligible_promo_item_definition_ids()`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InventoryEligiblePromoItemDefIds {
    /// The error from Steam, or `None` on success.
    pub error: Option<SteamError>,
    /// The user the list belongs to.
    pub steam_id: SteamId,
    /// How many eligible item definitions there are.
    pub count: i32,
    /// `true` when Steam answered from its cache instead of the server.
    pub cached: bool,
}

impl DecodeCallResult<sys::SteamInventoryEligiblePromoItemDefIDs_t>
    for InventoryEligiblePromoItemDefIds
{
    fn decode(raw: &sys::SteamInventoryEligiblePromoItemDefIDs_t) -> Self {
        // SAFETY: CSteamID is a union; reading its 64-bit form is the documented layout.
        let steam_id = unsafe { raw.m_steamID.m_steamid.m_unAll64Bits };
        Self {
            error: error_of(raw.m_result),
            steam_id: SteamId(steam_id),
            count: raw.m_numEligiblePromoItemDefs,
            cached: raw.m_bCachedData,
        }
    }

    fn error(&self) -> Option<SteamError> {
        self.error.clone()
    }
}

impl_callback!(cb: SteamInventoryEligiblePromoItemDefIDs_t => InventoryEligiblePromoItemDefIds {
    <Self as DecodeCallResult<_>>::decode(&cb)
});

/// The result of [`Inventory::start_purchase()`]. The purchase itself finishes later, in the Steam
/// overlay, and its completion arrives as [`InventoryResultReady`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InventoryStartPurchaseResult {
    /// The error from Steam, or `None` when the purchase transaction was started.
    pub error: Option<SteamError>,
    /// The order id Steam assigned to the purchase.
    pub order_id: u64,
    /// The transaction id Steam assigned to the purchase.
    pub transaction_id: u64,
}

impl DecodeCallResult<sys::SteamInventoryStartPurchaseResult_t> for InventoryStartPurchaseResult {
    fn decode(raw: &sys::SteamInventoryStartPurchaseResult_t) -> Self {
        Self {
            error: error_of(raw.m_result),
            order_id: raw.m_ulOrderID,
            transaction_id: raw.m_ulTransID,
        }
    }

    fn error(&self) -> Option<SteamError> {
        self.error.clone()
    }
}

impl_callback!(cb: SteamInventoryStartPurchaseResult_t => InventoryStartPurchaseResult {
    <Self as DecodeCallResult<_>>::decode(&cb)
});

/// The result of [`Inventory::request_prices()`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InventoryRequestPricesResult {
    /// The error from Steam, or `None` when the prices were received.
    pub error: Option<SteamError>,
    /// The currency code the prices are in, for example `USD` (as Steam reports it).
    pub currency: String,
}

impl DecodeCallResult<sys::SteamInventoryRequestPricesResult_t> for InventoryRequestPricesResult {
    fn decode(raw: &sys::SteamInventoryRequestPricesResult_t) -> Self {
        let bytes: Vec<u8> = raw.m_rgchCurrency.iter().map(|&c| c as u8).collect();
        Self {
            error: error_of(raw.m_result),
            currency: c_bytes_to_string(&bytes),
        }
    }

    fn error(&self) -> Option<SteamError> {
        self.error.clone()
    }
}

impl_callback!(cb: SteamInventoryRequestPricesResult_t => InventoryRequestPricesResult {
    <Self as DecodeCallResult<_>>::decode(&cb)
});

impl Inventory {
    /// Captures the whole inventory of the current user.
    ///
    /// Steam may rate-limit this call and return a cached copy, so call it when the inventory is
    /// about to be shown, not on every frame. Returns `None` when the inventory is unavailable.
    /// The result is ready when [`InventoryResultReady`] arrives for it.
    pub fn get_all_items(&self) -> Option<InventoryResult> {
        let mut handle = RESULT_INVALID;
        let ok = unsafe { sys::SteamAPI_ISteamInventory_GetAllItems(self.inventory, &mut handle) };
        adopt_result(self.inventory, handle, ok)
    }

    /// Captures the state of some items of the current user, identified by their instance ids.
    ///
    /// The result can be serialized and sent to other players to prove ownership of those items
    /// without exposing the whole inventory. Returns `None` when Steam refuses the call. The result
    /// is ready when [`InventoryResultReady`] arrives for it.
    pub fn get_items_by_id(&self, item_ids: &[u64]) -> Option<InventoryResult> {
        let count = len_u32(item_ids.len())?;
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_GetItemsByID(
                self.inventory,
                &mut handle,
                item_ids.as_ptr(),
                count,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Verifies and reads a result that was serialized by [`InventoryResult::serialize()`].
    ///
    /// This call is synchronous: the SDK documents no [`InventoryResultReady`] for it. Read
    /// [`InventoryResult::status()`] right away. An expired result is still returned, with status
    /// `Expired`. Returns `None` when Steam rejects the data, or when the call needs a full online
    /// verification and Steam is offline. Check the owner with
    /// [`InventoryResult::check_steam_id()`] before you trust the items.
    pub fn deserialize_result(&self, data: &[u8]) -> Option<InventoryResult> {
        let len = len_u32(data.len())?;
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_DeserializeResult(
                self.inventory,
                &mut handle,
                data.as_ptr() as *const c_void,
                len,
                false,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Creates new items of the given definitions. Each pair is `(definition, quantity)`.
    ///
    /// Steam accepts this only from accounts in the publisher group of the game, and only for
    /// prototyping. It creates items in the user's real inventory. The result is ready when
    /// [`InventoryResultReady`] arrives for it.
    pub fn generate_items(&self, items: &[(i32, u32)]) -> Option<InventoryResult> {
        let (defs, quantities) = split_pairs(items);
        let count = len_u32(defs.len())?;
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_GenerateItems(
                self.inventory,
                &mut handle,
                defs.as_ptr(),
                quantities.as_ptr(),
                count,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Grants the promotional items for which the user is eligible, once only. Steam checks every
    /// promotion. Being eligible for none is still a success, with an empty result.
    ///
    /// This changes the user's real inventory. The result is ready when [`InventoryResultReady`]
    /// arrives for it.
    pub fn grant_promo_items(&self) -> Option<InventoryResult> {
        let mut handle = RESULT_INVALID;
        let ok =
            unsafe { sys::SteamAPI_ISteamInventory_GrantPromoItems(self.inventory, &mut handle) };
        adopt_result(self.inventory, handle, ok)
    }

    /// Grants one promotional item definition, if the user is eligible for it.
    ///
    /// This changes the user's real inventory. The result is ready when [`InventoryResultReady`]
    /// arrives for it.
    pub fn add_promo_item(&self, definition: i32) -> Option<InventoryResult> {
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_AddPromoItem(self.inventory, &mut handle, definition)
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Grants several promotional item definitions, if the user is eligible for them. Only the
    /// listed definitions are checked.
    ///
    /// This changes the user's real inventory. The result is ready when [`InventoryResultReady`]
    /// arrives for it.
    pub fn add_promo_items(&self, definitions: &[i32]) -> Option<InventoryResult> {
        let count = len_u32(definitions.len())?;
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_AddPromoItems(
                self.inventory,
                &mut handle,
                definitions.as_ptr(),
                count,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Permanently removes `quantity` of one item instance from the user's inventory. The items
    /// cannot be recovered. The result is ready when [`InventoryResultReady`] arrives for it.
    ///
    /// This changes the user's real inventory and is not reversible. Ask the user to confirm first.
    pub fn consume_item(&self, item_id: u64, quantity: u32) -> Option<InventoryResult> {
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_ConsumeItem(
                self.inventory,
                &mut handle,
                item_id,
                quantity,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Atomically destroys the `destroy` items and generates the `generate` items, following the
    /// exchange recipe defined for them in Steamworks. Each pair is `(definition or item id, quantity)`.
    ///
    /// Steam fails the whole exchange when the items do not match a recipe or the quantities are too
    /// small. This changes the user's real inventory and is not reversible. Ask the user to confirm
    /// first. The result is ready when [`InventoryResultReady`] arrives for it.
    pub fn exchange_items(
        &self,
        generate: &[(i32, u32)],
        destroy: &[(u64, u32)],
    ) -> Option<InventoryResult> {
        let (gen_defs, gen_quantities) = split_pairs(generate);
        let (destroy_ids, destroy_quantities) = split_pairs(destroy);
        let gen_count = len_u32(gen_defs.len())?;
        let destroy_count = len_u32(destroy_ids.len())?;
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_ExchangeItems(
                self.inventory,
                &mut handle,
                gen_defs.as_ptr(),
                gen_quantities.as_ptr(),
                gen_count,
                destroy_ids.as_ptr(),
                destroy_quantities.as_ptr(),
                destroy_count,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Moves `quantity` from the stack `source` into the stack `destination`. Pass
    /// [`ITEM_INSTANCE_ID_INVALID`] as the destination to split the stack into a new item.
    ///
    /// Only stackable items can be split or merged. This changes the user's real inventory. The
    /// result is ready when [`InventoryResultReady`] arrives for it.
    pub fn transfer_item_quantity(
        &self,
        source: u64,
        quantity: u32,
        destination: u64,
    ) -> Option<InventoryResult> {
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_TransferItemQuantity(
                self.inventory,
                &mut handle,
                source,
                quantity,
                destination,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Deprecated by Steam. Sending a heartbeat is not required for playtime accounting; Steam no
    /// longer needs it.
    pub fn send_item_drop_heartbeat(&self) {
        unsafe { sys::SteamAPI_ISteamInventory_SendItemDropHeartbeat(self.inventory) }
    }

    /// Turns playtime credit into an item drop from the given drop list definition. Only definitions
    /// marked as playtime item generators in Steamworks can drop.
    ///
    /// Steam returns an empty result when there is not enough playtime credit. This changes the
    /// user's real inventory. Players can modify the client, so do not use the definition to control
    /// rarity. The result is ready when [`InventoryResultReady`] arrives for it.
    pub fn trigger_item_drop(&self, drop_list_definition: i32) -> Option<InventoryResult> {
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_TriggerItemDrop(
                self.inventory,
                &mut handle,
                drop_list_definition,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Deprecated by Steam: the SDK documents this call as not supported. It is wrapped for
    /// completeness and is expected to fail. Unverified against a live Steam client.
    ///
    /// Each pair is `(item id, quantity)`. `give` lists what the user gives, `get` what the partner gives.
    pub fn trade_items(
        &self,
        partner: SteamId,
        give: &[(u64, u32)],
        get: &[(u64, u32)],
    ) -> Option<InventoryResult> {
        let (give_ids, give_quantities) = split_pairs(give);
        let (get_ids, get_quantities) = split_pairs(get);
        let give_count = len_u32(give_ids.len())?;
        let get_count = len_u32(get_ids.len())?;
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_TradeItems(
                self.inventory,
                &mut handle,
                partner.0,
                give_ids.as_ptr(),
                give_quantities.as_ptr(),
                give_count,
                get_ids.as_ptr(),
                get_quantities.as_ptr(),
                get_count,
            )
        };
        adopt_result(self.inventory, handle, ok)
    }

    /// Starts loading and refreshing the item definitions. Returns `false` when the load cannot start.
    ///
    /// Steam announces the loaded definitions with [`InventoryDefinitionUpdate`]. Read them after that
    /// callback, not right after this call.
    pub fn load_item_definitions(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamInventory_LoadItemDefinitions(self.inventory) }
    }

    /// Returns the ids of all item definitions configured in Steamworks. The ids are not necessarily
    /// contiguous. Returns `None` when Steam refuses the call (for example before the definitions load).
    pub fn item_definition_ids(&self) -> Option<Vec<i32>> {
        unsafe {
            let mut count = 0u32;
            if !sys::SteamAPI_ISteamInventory_GetItemDefinitionIDs(
                self.inventory,
                ptr::null_mut(),
                &mut count,
            ) {
                return None;
            }
            let mut ids = vec![0i32; count as usize];
            let mut written = count;
            if !sys::SteamAPI_ISteamInventory_GetItemDefinitionIDs(
                self.inventory,
                ids.as_mut_ptr(),
                &mut written,
            ) {
                return None;
            }
            ids.truncate(written.min(count) as usize);
            Some(ids)
        }
    }

    /// Reads a string property of an item definition. Some properties, such as `name`, are localized
    /// for the current Steam language. Returns `None` when the name contains a NUL byte, when the
    /// property is unknown, or when Steam refuses the read.
    pub fn item_definition_property(&self, definition: i32, name: &str) -> Option<String> {
        let name = CString::new(name).ok()?;
        read_string(|buf, size| unsafe {
            sys::SteamAPI_ISteamInventory_GetItemDefinitionProperty(
                self.inventory,
                definition,
                name.as_ptr(),
                buf,
                size,
            )
        })
    }

    /// Returns the names of the properties an item definition has. Returns `None` when Steam refuses
    /// the call.
    pub fn item_definition_property_names(&self, definition: i32) -> Option<Vec<String>> {
        read_string(|buf, size| unsafe {
            sys::SteamAPI_ISteamInventory_GetItemDefinitionProperty(
                self.inventory,
                definition,
                ptr::null(),
                buf,
                size,
            )
        })
        .map(|list| split_property_names(&list))
    }

    /// Asks Steam which promotional item definitions the user can be granted manually, through
    /// [`add_promo_item()`](Self::add_promo_item). Read the ids with
    /// [`eligible_promo_item_definition_ids()`](Self::eligible_promo_item_definition_ids) after `cb`.
    ///
    /// This does not change the inventory. `cb` receives `Err` when the request fails.
    pub fn request_eligible_promo_item_definitions<F>(&self, steam_id: SteamId, cb: F)
    where
        F: FnOnce(Result<InventoryEligiblePromoItemDefIds, SteamError>) + 'static + Send,
    {
        let api_call = unsafe {
            sys::SteamAPI_ISteamInventory_RequestEligiblePromoItemDefinitionsIDs(
                self.inventory,
                steam_id.0,
            )
        };
        unsafe { register_result(&self.inner, api_call, cb) }
    }

    /// Returns the ids of the eligible promotional item definitions of `steam_id`. Pass the `count`
    /// from the [`InventoryEligiblePromoItemDefIds`] that the request returned.
    ///
    /// Returns `None` when Steam refuses the call. An empty list is returned without asking Steam.
    pub fn eligible_promo_item_definition_ids(
        &self,
        steam_id: SteamId,
        count: u32,
    ) -> Option<Vec<i32>> {
        if count == 0 {
            return Some(Vec::new());
        }
        let mut ids = vec![0i32; count as usize];
        let mut written = count;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_GetEligiblePromoItemDefinitionIDs(
                self.inventory,
                steam_id.0,
                ids.as_mut_ptr(),
                &mut written,
            )
        };
        if !ok {
            return None;
        }
        ids.truncate(written.min(count) as usize);
        Some(ids)
    }

    /// Starts buying the given item definitions. Each pair is `(definition, quantity)`.
    ///
    /// The user completes the purchase in the Steam overlay; this call only starts the transaction.
    /// This is a real purchase that charges the user. `cb` receives the order and transaction ids,
    /// or `Err` when Steam cannot start the transaction. The purchase result itself arrives later as
    /// [`InventoryResultReady`].
    pub fn start_purchase<F>(&self, items: &[(i32, u32)], cb: F)
    where
        F: FnOnce(Result<InventoryStartPurchaseResult, SteamError>) + 'static + Send,
    {
        let (defs, quantities) = split_pairs(items);
        let count = match len_u32(defs.len()) {
            Some(count) => count,
            None => {
                cb(Err(SteamError::InvalidParameter));
                return;
            }
        };
        let api_call = unsafe {
            sys::SteamAPI_ISteamInventory_StartPurchase(
                self.inventory,
                defs.as_ptr(),
                quantities.as_ptr(),
                count,
            )
        };
        unsafe { register_result(&self.inner, api_call, cb) }
    }

    /// Asks Steam for the current prices of all item definitions that have one. Call this before
    /// [`items_with_prices()`](Self::items_with_prices) or [`item_price()`](Self::item_price).
    ///
    /// `cb` receives the currency of the prices, or `Err` when the request fails.
    pub fn request_prices<F>(&self, cb: F)
    where
        F: FnOnce(Result<InventoryRequestPricesResult, SteamError>) + 'static + Send,
    {
        let api_call = unsafe { sys::SteamAPI_ISteamInventory_RequestPrices(self.inventory) };
        unsafe { register_result(&self.inner, api_call, cb) }
    }

    /// Returns how many item definitions have a price, after [`request_prices()`](Self::request_prices).
    pub fn num_items_with_prices(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamInventory_GetNumItemsWithPrices(self.inventory) }
    }

    /// Returns the prices of all item definitions that have one, after
    /// [`request_prices()`](Self::request_prices). Returns `None` when Steam refuses the call (for
    /// example before the prices were requested).
    pub fn items_with_prices(&self) -> Option<Vec<ItemPrice>> {
        let count = self.num_items_with_prices() as usize;
        let mut defs = vec![0i32; count];
        let mut current = vec![0u64; count];
        let mut base = vec![0u64; count];
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_GetItemsWithPrices(
                self.inventory,
                defs.as_mut_ptr(),
                current.as_mut_ptr(),
                base.as_mut_ptr(),
                count as u32,
            )
        };
        if !ok {
            return None;
        }
        Some(
            defs.into_iter()
                .zip(current)
                .zip(base)
                .map(|((definition, current), base)| ItemPrice {
                    definition,
                    current,
                    base,
                })
                .collect(),
        )
    }

    /// Returns the price of one item definition, after [`request_prices()`](Self::request_prices).
    /// Returns `None` when there is no stored price for it.
    pub fn item_price(&self, definition: i32) -> Option<ItemPrice> {
        let mut current = 0u64;
        let mut base = 0u64;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_GetItemPrice(
                self.inventory,
                definition,
                &mut current,
                &mut base,
            )
        };
        if ok {
            Some(ItemPrice {
                definition,
                current,
                base,
            })
        } else {
            None
        }
    }

    /// Starts a batch of item property changes. Returns `None` when Steam refuses to start one.
    ///
    /// Nothing changes until the batch is submitted with [`PropertyUpdate::submit()`].
    pub fn start_update_properties(&self) -> Option<PropertyUpdate> {
        let handle = unsafe { sys::SteamAPI_ISteamInventory_StartUpdateProperties(self.inventory) };
        if handle == UPDATE_HANDLE_INVALID {
            None
        } else {
            Some(PropertyUpdate {
                inventory: self.inventory,
                handle,
            })
        }
    }

    /// Asks Steam to fetch the item described by an inspection token, for example one from an item
    /// link. The result is ready when [`InventoryResultReady`] arrives for it. Returns `None` when the
    /// token contains a NUL byte or Steam refuses the call.
    pub fn inspect_item(&self, token: &str) -> Option<InventoryResult> {
        let token = CString::new(token).ok()?;
        let mut handle = RESULT_INVALID;
        let ok = unsafe {
            sys::SteamAPI_ISteamInventory_InspectItem(self.inventory, &mut handle, token.as_ptr())
        };
        adopt_result(self.inventory, handle, ok)
    }
}

/// Wraps a result handle that a producer wrote. A handle is destroyed here when the call failed, so
/// a failed call that still wrote a handle does not leak it.
fn adopt_result(
    inventory: *mut sys::ISteamInventory,
    handle: sys::SteamInventoryResult_t,
    ok: bool,
) -> Option<InventoryResult> {
    let result = if handle > 0 {
        Some(InventoryResult { inventory, handle })
    } else {
        None
    };
    if ok {
        result
    } else {
        None
    }
}

/// Registers a call result and reports its decoded value, or `SteamError` when the call failed.
///
/// # Safety
///
/// `api_call` must come from a call on the inventory interface of the running Steam client.
unsafe fn register_result<S, T, F>(inner: &Arc<Inner>, api_call: sys::SteamAPICall_t, cb: F)
where
    T: DecodeCallResult<S> + 'static,
    S: 'static,
    F: FnOnce(Result<T, SteamError>) + 'static + Send,
{
    if api_call == 0 {
        // The SDK returns k_uAPICallInvalid when the call cannot be made at all. Nothing will arrive.
        cb(Err(SteamError::Generic));
        return;
    }
    register_call_result::<S, _>(inner, api_call, move |raw, io_error| {
        cb(if io_error {
            Err(SteamError::IOFailure)
        } else {
            let value = T::decode(raw);
            match value.error() {
                Some(error) => Err(error),
                None => Ok(value),
            }
        })
    });
}

/// Maps a call result code to `None` for success, or to the error otherwise.
fn error_of(result: sys::EResult) -> Option<SteamError> {
    match result {
        sys::EResult::k_EResultOK => None,
        error => Some(error.into()),
    }
}

/// Converts a length to the `u32` the SDK takes, or `None` when it does not fit.
fn len_u32(len: usize) -> Option<u32> {
    u32::try_from(len).ok()
}

/// Splits `(value, quantity)` pairs into the two parallel arrays the SDK takes.
fn split_pairs<T: Copy>(pairs: &[(T, u32)]) -> (Vec<T>, Vec<u32>) {
    pairs
        .iter()
        .map(|&(value, quantity)| (value, quantity))
        .unzip()
}

/// Reads a string the way the SDK documents it: a first call with a NULL buffer reports the size,
/// then a second call copies the value into a buffer of that size.
fn read_string(mut call: impl FnMut(*mut c_char, *mut u32) -> bool) -> Option<String> {
    let mut size = 0u32;
    if !call(ptr::null_mut(), &mut size) {
        return None;
    }
    let mut buf = vec![0u8; size as usize + 1];
    let mut written = buf.len() as u32;
    if !call(buf.as_mut_ptr() as *mut c_char, &mut written) {
        return None;
    }
    Some(c_bytes_to_string(&buf[..(written as usize).min(buf.len())]))
}

/// Takes the bytes up to the first NUL and decodes them as UTF-8, replacing invalid sequences.
fn c_bytes_to_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Splits the comma-separated property name list Steam returns for a NULL name. Empty entries are
/// dropped.
fn split_property_names(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Maps the result code of an inventory-ready callback to an error. An expired result still has
/// items, so it is not reported as an error.
fn ready_error(result: sys::EResult) -> Option<SteamError> {
    match result {
        sys::EResult::k_EResultOK | sys::EResult::k_EResultExpired => None,
        error => Some(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_details_copy_every_field() {
        let raw = sys::SteamItemDetails_t {
            m_itemId: 0x1234_5678_9abc_def0,
            m_iDefinition: 42,
            m_unQuantity: 7,
            m_unFlags: ITEM_FLAG_NO_TRADE | ITEM_FLAG_CONSUMED,
        };
        let item = ItemDetails::from(raw);
        assert_eq!(item.item_id, 0x1234_5678_9abc_def0);
        assert_eq!(item.definition, 42);
        assert_eq!(item.quantity, 7);
        assert_eq!(item.flags, ITEM_FLAG_NO_TRADE | ITEM_FLAG_CONSUMED);
        assert_eq!(item.flags & ITEM_FLAG_REMOVED, 0);
    }

    #[test]
    fn c_strings_stop_at_the_first_nul() {
        assert_eq!(c_bytes_to_string(b"USD\0\0"), "USD");
        assert_eq!(c_bytes_to_string(b"no nul"), "no nul");
        assert_eq!(c_bytes_to_string(b""), "");
        assert_eq!(c_bytes_to_string(&[b'a', 0xff, b'b', 0]), "a\u{fffd}b");
    }

    #[test]
    fn property_name_lists_drop_empty_entries() {
        assert_eq!(
            split_property_names("name, description,,price_tier ,"),
            vec!["name", "description", "price_tier"]
        );
        assert!(split_property_names("").is_empty());
    }

    #[test]
    fn ready_callback_treats_expired_as_readable() {
        assert!(ready_error(sys::EResult::k_EResultOK).is_none());
        assert!(ready_error(sys::EResult::k_EResultExpired).is_none());
        assert!(matches!(
            ready_error(sys::EResult::k_EResultFail),
            Some(SteamError::Generic)
        ));
    }

    #[test]
    fn pairs_split_into_parallel_arrays() {
        let (ids, quantities) = split_pairs(&[(10i32, 1u32), (20, 5)]);
        assert_eq!(ids, vec![10, 20]);
        assert_eq!(quantities, vec![1, 5]);
        let (empty_ids, empty_quantities) = split_pairs::<u64>(&[]);
        assert!(empty_ids.is_empty() && empty_quantities.is_empty());
    }

    #[test]
    fn lengths_beyond_u32_are_rejected() {
        assert_eq!(len_u32(3), Some(3));
        assert_eq!(len_u32(u32::MAX as usize + 1), None);
    }

    #[test]
    fn call_results_map_steam_errors() {
        assert!(error_of(sys::EResult::k_EResultOK).is_none());
        assert!(matches!(
            error_of(sys::EResult::k_EResultLimitExceeded),
            Some(SteamError::LimitExceeded)
        ));
    }
}
