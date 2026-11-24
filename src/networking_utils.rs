use crate::networking_types::{
    NetworkingAvailabilityResult, NetworkingConfigDataType, NetworkingIdentity, NetworkingMessage,
};
use crate::{register_callback, Callback, Inner, SteamError};
use std::convert::TryInto;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::Arc;
use std::time::Duration;

use steamworks_sys as sys;

/// Access to the steam networking sockets interface
pub struct NetworkingUtils {
    pub(crate) utils: *mut sys::ISteamNetworkingUtils,
    pub(crate) inner: Arc<Inner>,
}

unsafe impl Send for NetworkingUtils {}
unsafe impl Sync for NetworkingUtils {}

impl NetworkingUtils {
    /// Allocate and initialize a message object.  Usually the reason
    /// you call this is to pass it to ISteamNetworkingSockets::SendMessages.
    /// The returned object will have all of the relevant fields cleared to zero.
    ///
    /// Optionally you can also request that this system allocate space to
    /// hold the payload itself.  If cbAllocateBuffer is nonzero, the system
    /// will allocate memory to hold a payload of at least cbAllocateBuffer bytes.
    /// m_pData will point to the allocated buffer, m_cbSize will be set to the
    /// size, and m_pfnFreeData will be set to the proper function to free up
    /// the buffer.
    ///
    /// If cbAllocateBuffer=0, then no buffer is allocated.  m_pData will be NULL,
    /// m_cbSize will be zero, and m_pfnFreeData will be NULL.  You will need to
    /// set each of these.
    pub fn allocate_message(&self, buffer_size: usize) -> NetworkingMessage {
        unsafe {
            let message =
                sys::SteamAPI_ISteamNetworkingUtils_AllocateMessage(self.utils, buffer_size as _);
            NetworkingMessage {
                message,
                _inner: self.inner.clone(),
            }
        }
    }

    /// If you know that you are going to be using the relay network (for example,
    /// because you anticipate making P2P connections), call this to initialize the
    /// relay network.  If you do not call this, the initialization will
    /// be delayed until the first time you use a feature that requires access
    /// to the relay network, which will delay that first access.
    ///
    /// You can also call this to force a retry if the previous attempt has failed.
    /// Performing any action that requires access to the relay network will also
    /// trigger a retry, and so calling this function is never strictly necessary,
    /// but it can be useful to call it a program launch time, if access to the
    /// relay network is anticipated.
    ///
    /// Use GetRelayNetworkStatus or listen for SteamRelayNetworkStatus_t
    /// callbacks to know when initialization has completed.
    /// Typically initialization completes in a few seconds.
    ///
    /// Note: dedicated servers hosted in known data centers do *not* need
    /// to call this, since they do not make routing decisions.  However, if
    /// the dedicated server will be using P2P functionality, it will act as
    /// a "client" and this should be called.
    pub fn init_relay_network_access(&self) {
        unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_InitRelayNetworkAccess(self.utils);
        }
    }

    /// Fetch current status of the relay network.
    ///
    /// If you want more detailed information use [`detailed_relay_network_status`](#method.detailed_relay_network_status) instead.
    pub fn relay_network_status(&self) -> NetworkingAvailabilityResult {
        unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_GetRelayNetworkStatus(
                self.utils,
                std::ptr::null_mut(),
            )
            .try_into()
        }
    }

    /// Fetch current detailed status of the relay network.
    pub fn detailed_relay_network_status(&self) -> RelayNetworkStatus {
        unsafe {
            let mut status = sys::SteamRelayNetworkStatus_t {
                m_eAvail: sys::ESteamNetworkingAvailability::k_ESteamNetworkingAvailability_Unknown,
                m_bPingMeasurementInProgress: 0,
                m_eAvailNetworkConfig:
                    sys::ESteamNetworkingAvailability::k_ESteamNetworkingAvailability_Unknown,
                m_eAvailAnyRelay:
                    sys::ESteamNetworkingAvailability::k_ESteamNetworkingAvailability_Unknown,
                m_debugMsg: [0; 256],
            };
            sys::SteamAPI_ISteamNetworkingUtils_GetRelayNetworkStatus(self.utils, &mut status);
            status.into()
        }
    }

    /// Register the callback for relay network status updates.
    ///
    /// Calling this more than once replaces the previous callback.
    pub fn relay_network_status_callback(
        &self,
        mut callback: impl FnMut(RelayNetworkStatus) + Send + 'static,
    ) {
        unsafe {
            register_callback(&self.inner, move |status: RelayNetworkStatusCallback| {
                callback(status.status);
            });
        }
    }
}

/// A point on Valve's network, used to estimate latency between two hosts without sending packets.
///
/// It is only meaningful in the process that made it. To share one, convert it with [`NetworkingUtils::ping_location_to_string`] and parse it back with [`NetworkingUtils::parse_ping_location`]; do not parse the text yourself.
#[derive(Clone, Copy)]
pub struct NetworkingPingLocation {
    inner: sys::SteamNetworkPingLocation_t,
}

impl NetworkingPingLocation {
    fn zeroed() -> Self {
        Self {
            inner: unsafe { std::mem::zeroed() },
        }
    }
}

/// An IP address and port in Steam's networking address format. It can also hold one of Steam's fake IPs.
#[derive(Clone, Copy)]
pub struct NetworkingIpAddr {
    inner: sys::SteamNetworkingIPAddr,
}

impl NetworkingIpAddr {
    fn zeroed() -> Self {
        Self {
            inner: unsafe { std::mem::zeroed() },
        }
    }

    fn as_ptr(&self) -> *const sys::SteamNetworkingIPAddr {
        &self.inner
    }
}

/// Classification of an IPv4 address or address structure against Steam's fake IP range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeIpType {
    /// The value is not an IP address, or the SDK does not know this value.
    Invalid,
    /// A valid IP address outside the reserved fake range.
    NotFake,
    /// A globally unique IPv4 address for this app. Steam manages the address space.
    GlobalIpv4,
    /// A locally unique IPv4 address. The local process manages it, and it must not be shared.
    LocalIpv4,
}

impl From<sys::ESteamNetworkingFakeIPType> for FakeIpType {
    fn from(value: sys::ESteamNetworkingFakeIPType) -> Self {
        match value {
            sys::ESteamNetworkingFakeIPType::k_ESteamNetworkingFakeIPType_NotFake => Self::NotFake,
            sys::ESteamNetworkingFakeIPType::k_ESteamNetworkingFakeIPType_GlobalIPv4 => {
                Self::GlobalIpv4
            }
            sys::ESteamNetworkingFakeIPType::k_ESteamNetworkingFakeIPType_LocalIPv4 => {
                Self::LocalIpv4
            }
            _ => Self::Invalid,
        }
    }
}

/// The value of a networking config setting, read with [`NetworkingUtils::get_global_config_value`] or [`NetworkingUtils::get_connection_config_value`].
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkingConfigValueData {
    /// A 32-bit integer setting.
    Int32(i32),
    /// A 64-bit integer setting.
    Int64(i64),
    /// A float setting.
    Float(f32),
    /// A text setting, decoded as UTF-8 with invalid bytes replaced.
    String(String),
}

/// The scope a config value can be set at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigValueScope {
    /// Global defaults for the whole process.
    Global,
    /// One networking sockets interface.
    SocketsInterface,
    /// One listen socket.
    ListenSocket,
    /// One connection.
    Connection,
}

/// Name, data type and scope of a config value, from [`NetworkingUtils::config_value_info`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkingConfigInfo {
    /// The SDK name of the value, for example `TimeoutInitial`.
    pub name: String,
    /// The type of value the setting holds.
    pub data_type: NetworkingConfigDataType,
    /// The scope the value can be set at.
    pub scope: ConfigValueScope,
}

/// Returns the config value for a raw number of Steam's `ESteamNetworkingConfigValue`, or `None` when the SDK header does not list that number.
///
/// This is the lookup that makes numbers usable with the config functions without unsafe casts.
pub fn config_value_from_raw(raw: i32) -> Option<sys::ESteamNetworkingConfigValue> {
    CONFIG_VALUES
        .iter()
        .copied()
        .find(|value| *value as i32 == raw)
}

/// Returns the readable code of a Valve data center (POP) ID.
///
/// Steam packs the code into the ID: the first three characters go in the low three bytes, and the fourth (if any) in the top byte. Codes are ASCII, usually three or four characters.
/// This does not call the SDK. The byte layout comes from `steamnetworkingtypes.h`.
pub fn pop_code(pop: u32) -> String {
    [
        (pop >> 16) as u8,
        (pop >> 8) as u8,
        pop as u8,
        (pop >> 24) as u8,
    ]
    .iter()
    .take_while(|&&byte| byte != 0)
    .map(|&byte| byte as char)
    .collect()
}

fn non_negative(value: i32) -> Option<i32> {
    if value >= 0 {
        Some(value)
    } else {
        None
    }
}

fn bool_result(ok: bool) -> Result<(), ()> {
    if ok {
        Ok(())
    } else {
        Err(())
    }
}

fn c_buffer_to_string(buffer: &[c_char]) -> String {
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|&&byte| byte != 0)
        .map(|&byte| byte as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn data_type_from_sys(
    value: sys::ESteamNetworkingConfigDataType,
) -> Option<NetworkingConfigDataType> {
    Some(match value {
        sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_Int32 => {
            NetworkingConfigDataType::Int32
        }
        sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_Int64 => {
            NetworkingConfigDataType::Int64
        }
        sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_Float => {
            NetworkingConfigDataType::Float
        }
        sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_String => {
            NetworkingConfigDataType::String
        }
        sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_Ptr => {
            NetworkingConfigDataType::Callback
        }
        _ => return None,
    })
}

fn scope_from_sys(value: sys::ESteamNetworkingConfigScope) -> ConfigValueScope {
    match value {
        sys::ESteamNetworkingConfigScope::k_ESteamNetworkingConfig_SocketsInterface => {
            ConfigValueScope::SocketsInterface
        }
        sys::ESteamNetworkingConfigScope::k_ESteamNetworkingConfig_ListenSocket => {
            ConfigValueScope::ListenSocket
        }
        sys::ESteamNetworkingConfigScope::k_ESteamNetworkingConfig_Connection => {
            ConfigValueScope::Connection
        }
        _ => ConfigValueScope::Global,
    }
}

// Every ESteamNetworkingConfigValue in steamnetworkingtypes.h except k_ESteamNetworkingConfig_Invalid.
// Generated from the bindings; add new entries here when the SDK adds them.
const CONFIG_VALUES: &[sys::ESteamNetworkingConfigValue] = &[
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_TimeoutInitial,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_TimeoutConnected,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SendBufferSize,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_RecvBufferSize,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_RecvBufferMessages,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_RecvMaxMessageSize,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_RecvMaxSegmentsPerPacket,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_ConnectionUserData,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SendRateMin,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SendRateMax,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_NagleTime,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_IP_AllowWithoutAuth,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_IPLocalHost_AllowWithoutAuth,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_MTU_PacketSize,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_MTU_DataSize,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Unencrypted,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SymmetricConnect,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_LocalVirtualPort,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_DualWifi_Enable,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_EnableDiagnosticsUI,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SendTimeSincePreviousPacket,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketLoss_Send,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketLoss_Recv,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketLag_Send,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketLag_Recv,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketJitter_Send_Avg,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketJitter_Send_Max,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketJitter_Send_Pct,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketJitter_Recv_Avg,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketJitter_Recv_Max,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketJitter_Recv_Pct,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketReorder_Send,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketReorder_Recv,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketReorder_Time,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketDup_Send,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketDup_Recv,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakePacketDup_TimeMax,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_PacketTraceMaxBytes,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakeRateLimit_Send_Rate,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakeRateLimit_Send_Burst,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakeRateLimit_Recv_Rate,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_FakeRateLimit_Recv_Burst,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_OutOfOrderCorrectionWindowMicroseconds,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Callback_ConnectionStatusChanged,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Callback_AuthStatusChanged,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Callback_RelayNetworkStatusChanged,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Callback_MessagesSessionRequest,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Callback_MessagesSessionFailed,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Callback_CreateConnectionSignaling,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Callback_FakeIPResult,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_P2P_STUN_ServerList,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_P2P_Transport_ICE_Enable,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_P2P_Transport_ICE_Penalty,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_P2P_Transport_SDR_Penalty,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_P2P_TURN_ServerList,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_P2P_TURN_UserList,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_P2P_TURN_PassList,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_P2P_Transport_ICE_Implementation,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SDRClient_ConsecutitivePingTimeoutsFailInitial,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SDRClient_ConsecutitivePingTimeoutsFail,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SDRClient_MinPingsBeforePingAccurate,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SDRClient_SingleSocket,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SDRClient_ForceRelayCluster,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SDRClient_DevTicket,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SDRClient_ForceProxyAddr,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SDRClient_FakeClusterPing,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_SDRClient_LimitPingProbesToNearestN,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_LogLevel_AckRTT,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_LogLevel_PacketDecode,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_LogLevel_Message,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_LogLevel_PacketGaps,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_LogLevel_P2PRendezvous,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_LogLevel_SDRRelayPings,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_ECN,
    sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_DELETED_EnumerateDevVars,
];

impl NetworkingUtils {
    /// Returns where this host is on Valve's network, with the age of that data in seconds.
    ///
    /// Returns `None` when no ping data exists yet. Steam needs a few seconds after `init_relay_network_access()` before the data is ready.
    pub fn local_ping_location(&self) -> Option<(NetworkingPingLocation, f32)> {
        let mut location = NetworkingPingLocation::zeroed();
        let age = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_GetLocalPingLocation(
                self.utils,
                &mut location.inner,
            )
        };
        if age < 0.0 {
            None
        } else {
            Some((location, age))
        }
    }

    /// Converts a ping location to its text form. The text is compact and readable, but its format may change in later SDK versions, so do not parse it yourself.
    pub fn ping_location_to_string(&self, location: &NetworkingPingLocation) -> String {
        // k_cchMaxSteamNetworkingPingLocationString
        let mut buffer = vec![0 as c_char; 1024];
        unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_ConvertPingLocationToString(
                self.utils,
                &location.inner,
                buffer.as_mut_ptr(),
                buffer.len() as i32,
            );
        }
        c_buffer_to_string(&buffer)
    }

    /// Parses text made by [`NetworkingUtils::ping_location_to_string`]. Returns `None` when Steam does not understand the text or it contains a NUL byte.
    pub fn parse_ping_location(&self, text: &str) -> Option<NetworkingPingLocation> {
        let text = CString::new(text).ok()?;
        let mut location = NetworkingPingLocation::zeroed();
        let ok = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_ParsePingLocationString(
                self.utils,
                text.as_ptr(),
                &mut location.inner,
            )
        };
        if ok {
            Some(location)
        } else {
            None
        }
    }

    /// Estimates the round-trip time in milliseconds between two locations, using the relay network route.
    ///
    /// The estimate does not send packets. It is an upper bound when traffic can take a direct IP route. Returns `None` when Steam cannot estimate it (the SDK returns a negative code, either failed or unknown).
    pub fn estimate_ping_between_locations(
        &self,
        first: &NetworkingPingLocation,
        second: &NetworkingPingLocation,
    ) -> Option<i32> {
        let ms = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_EstimatePingTimeBetweenTwoLocations(
                self.utils,
                &first.inner,
                &second.inner,
            )
        };
        non_negative(ms)
    }

    /// Estimates the round-trip time in milliseconds from this host to `remote`. It is faster than [`NetworkingUtils::estimate_ping_between_locations`] when you compare many locations.
    ///
    /// Returns `None` when Steam cannot estimate it.
    pub fn estimate_ping_from_local_host(&self, remote: &NetworkingPingLocation) -> Option<i32> {
        let ms = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_EstimatePingTimeFromLocalHost(
                self.utils,
                &remote.inner,
            )
        };
        non_negative(ms)
    }

    /// Returns `true` when ping data at most `max_age` old already exists.
    ///
    /// Otherwise it returns `false` and starts a measurement if none is running. Only call it when you need a refresh now, for example on a user request, because Steam refreshes the data on its own. Watch the relay network status callback to see when a measurement finishes.
    pub fn check_ping_data_up_to_date(&self, max_age: Duration) -> bool {
        unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_CheckPingDataUpToDate(
                self.utils,
                max_age.as_secs_f32(),
            )
        }
    }

    /// Returns the best relayed ping to a data center, in milliseconds, with the POP the route uses. Returns `None` when Steam has no estimate.
    pub fn ping_to_data_center(&self, pop: u32) -> Option<(i32, u32)> {
        let mut via_pop: u32 = 0;
        let ms = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_GetPingToDataCenter(self.utils, pop, &mut via_pop)
        };
        non_negative(ms).map(|ms| (ms, via_pop))
    }

    /// Returns the direct ping, in milliseconds, to the relays at a data center, without relaying. Returns `None` when Steam has no estimate.
    pub fn direct_ping_to_pop(&self, pop: u32) -> Option<i32> {
        let ms = unsafe { sys::SteamAPI_ISteamNetworkingUtils_GetDirectPingToPOP(self.utils, pop) };
        non_negative(ms)
    }

    /// Returns the number of data centers (POPs) in the network config.
    pub fn pop_count(&self) -> usize {
        let count = unsafe { sys::SteamAPI_ISteamNetworkingUtils_GetPOPCount(self.utils) };
        usize::try_from(count).unwrap_or(0)
    }

    /// Returns the IDs of every data center in the network config. Use [`pop_code`] to get each readable code.
    pub fn pop_list(&self) -> Vec<u32> {
        let count = self.pop_count();
        let mut list = vec![0u32; count];
        let filled = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_GetPOPList(
                self.utils,
                list.as_mut_ptr(),
                count as i32,
            )
        };
        list.truncate(usize::try_from(filled).unwrap_or(0).min(count));
        list
    }

    /// Returns the local timestamp in microseconds. It never goes backwards and starts at about 30 days' worth of microseconds, so 0 is always in the past.
    ///
    /// The value only has meaning inside this run of the process; do not compare it with values from other machines or other runs.
    pub fn local_timestamp(&self) -> i64 {
        unsafe { sys::SteamAPI_ISteamNetworkingUtils_GetLocalTimestamp(self.utils) }
    }

    /// Returns `true` when an IPv4 address (as the SDK's `uint32`) is in Steam's fake range. The byte order of `ipv4` is unverified against a live Steam client.
    pub fn is_fake_ipv4(&self, ipv4: u32) -> bool {
        unsafe { sys::SteamAPI_ISteamNetworkingUtils_IsFakeIPv4(self.utils, ipv4) }
    }

    /// Classifies an IPv4 address (as the SDK's `uint32`) against the fake range. The byte order of `ipv4` is unverified against a live Steam client.
    pub fn fake_ipv4_type(&self, ipv4: u32) -> FakeIpType {
        unsafe { sys::SteamAPI_ISteamNetworkingUtils_GetIPv4FakeIPType(self.utils, ipv4).into() }
    }

    /// Returns the real identity behind a fake IP. Steam keeps the mapping for active connections and for a while after they close, so read it soon after the connection ends.
    ///
    /// Returns `Err` with the Steam error when the IP is not a fake IP (`InvalidParameter`) or the mapping is unknown (`NoMatch`).
    pub fn real_identity_for_fake_ip(
        &self,
        fake_ip: &NetworkingIpAddr,
    ) -> Result<NetworkingIdentity, SteamError> {
        let mut identity = NetworkingIdentity::new();
        let result = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_GetRealIdentityForFakeIP(
                self.utils,
                fake_ip.as_ptr(),
                identity.as_mut_ptr(),
            )
        };
        match result {
            sys::EResult::k_EResultOK => Ok(identity),
            other => Err(SteamError::from(other)),
        }
    }

    /// Sets a global `Int32` config value. `Err(())` when Steam refuses the value or the type does not match the setting.
    pub fn set_global_config_value_int32(
        &self,
        value: sys::ESteamNetworkingConfigValue,
        to: i32,
    ) -> Result<(), ()> {
        bool_result(unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SetGlobalConfigValueInt32(self.utils, value, to)
        })
    }

    /// Sets a global `Float` config value. `Err(())` when Steam refuses the value or the type does not match the setting.
    pub fn set_global_config_value_float(
        &self,
        value: sys::ESteamNetworkingConfigValue,
        to: f32,
    ) -> Result<(), ()> {
        bool_result(unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SetGlobalConfigValueFloat(self.utils, value, to)
        })
    }

    /// Sets a global `String` config value. `Err(())` when the text contains a NUL byte or Steam refuses it.
    pub fn set_global_config_value_string(
        &self,
        value: sys::ESteamNetworkingConfigValue,
        to: &str,
    ) -> Result<(), ()> {
        let to = CString::new(to).map_err(|_| ())?;
        bool_result(unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SetGlobalConfigValueString(
                self.utils,
                value,
                to.as_ptr(),
            )
        })
    }

    /// Sets an `Int32` config value on one connection. `Err(())` when the connection handle is invalid or Steam refuses the value.
    pub fn set_connection_config_value_int32(
        &self,
        connection: sys::HSteamNetConnection,
        value: sys::ESteamNetworkingConfigValue,
        to: i32,
    ) -> Result<(), ()> {
        bool_result(unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SetConnectionConfigValueInt32(
                self.utils, connection, value, to,
            )
        })
    }

    /// Sets a `Float` config value on one connection. `Err(())` when the connection handle is invalid or Steam refuses the value.
    pub fn set_connection_config_value_float(
        &self,
        connection: sys::HSteamNetConnection,
        value: sys::ESteamNetworkingConfigValue,
        to: f32,
    ) -> Result<(), ()> {
        bool_result(unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SetConnectionConfigValueFloat(
                self.utils, connection, value, to,
            )
        })
    }

    /// Sets a `String` config value on one connection. `Err(())` when the text contains a NUL byte, the connection handle is invalid, or Steam refuses the value.
    pub fn set_connection_config_value_string(
        &self,
        connection: sys::HSteamNetConnection,
        value: sys::ESteamNetworkingConfigValue,
        to: &str,
    ) -> Result<(), ()> {
        let to = CString::new(to).map_err(|_| ())?;
        bool_result(unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SetConnectionConfigValueString(
                self.utils,
                connection,
                value,
                to.as_ptr(),
            )
        })
    }

    /// Reads a global config value. Returns `None` when the value does not exist, has no global setting, or is a callback (a pointer the SDK does not let you read).
    pub fn get_global_config_value(
        &self,
        value: sys::ESteamNetworkingConfigValue,
    ) -> Option<NetworkingConfigValueData> {
        self.read_config_value(
            value,
            sys::ESteamNetworkingConfigScope::k_ESteamNetworkingConfig_Global,
            0,
        )
    }

    /// Reads a config value for one connection. It returns the inherited value when the connection has none of its own. Returns `None` when the value does not exist, the connection is invalid, or the value is a callback.
    pub fn get_connection_config_value(
        &self,
        connection: sys::HSteamNetConnection,
        value: sys::ESteamNetworkingConfigValue,
    ) -> Option<NetworkingConfigValueData> {
        self.read_config_value(
            value,
            sys::ESteamNetworkingConfigScope::k_ESteamNetworkingConfig_Connection,
            connection as isize,
        )
    }

    fn read_config_value(
        &self,
        value: sys::ESteamNetworkingConfigValue,
        scope: sys::ESteamNetworkingConfigScope,
        scope_object: isize,
    ) -> Option<NetworkingConfigValueData> {
        // Start with 256 bytes, 8-byte aligned. Steam reports the needed size when the buffer is too small, and we try once more.
        let mut capacity = 256usize;
        for _ in 0..2 {
            let mut buffer = vec![0u64; capacity.div_ceil(8)];
            let mut size = buffer.len() * 8;
            let mut data_type = sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_Int32;
            let result = unsafe {
                sys::SteamAPI_ISteamNetworkingUtils_GetConfigValue(
                    self.utils,
                    value,
                    scope,
                    scope_object,
                    &mut data_type,
                    buffer.as_mut_ptr() as *mut c_void,
                    &mut size,
                )
            };
            match result {
                sys::ESteamNetworkingGetConfigValueResult::k_ESteamNetworkingGetConfigValue_OK
                | sys::ESteamNetworkingGetConfigValueResult::k_ESteamNetworkingGetConfigValue_OKInherited => {
                    let bytes = unsafe {
                        std::slice::from_raw_parts(buffer.as_ptr() as *const u8, buffer.len() * 8)
                    };
                    let bytes = &bytes[..size.min(bytes.len())];
                    return match data_type {
                        sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_Int32 => {
                            Some(NetworkingConfigValueData::Int32(i32::from_ne_bytes(
                                bytes.get(..4)?.try_into().ok()?,
                            )))
                        }
                        sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_Int64 => {
                            Some(NetworkingConfigValueData::Int64(i64::from_ne_bytes(
                                bytes.get(..8)?.try_into().ok()?,
                            )))
                        }
                        sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_Float => {
                            Some(NetworkingConfigValueData::Float(f32::from_ne_bytes(
                                bytes.get(..4)?.try_into().ok()?,
                            )))
                        }
                        sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_String => {
                            let bytes: Vec<u8> = bytes.iter().copied().take_while(|&b| b != 0).collect();
                            Some(NetworkingConfigValueData::String(
                                String::from_utf8_lossy(&bytes).into_owned(),
                            ))
                        }
                        _ => None,
                    };
                }
                sys::ESteamNetworkingGetConfigValueResult::k_ESteamNetworkingGetConfigValue_BufferTooSmall => {
                    capacity = size.max(capacity + 1);
                }
                _ => return None,
            }
        }
        None
    }

    /// Returns the name, data type and scope of a config value. Returns `None` when the value does not exist.
    pub fn config_value_info(
        &self,
        value: sys::ESteamNetworkingConfigValue,
    ) -> Option<NetworkingConfigInfo> {
        let mut data_type = sys::ESteamNetworkingConfigDataType::k_ESteamNetworkingConfig_Int32;
        let mut scope = sys::ESteamNetworkingConfigScope::k_ESteamNetworkingConfig_Global;
        let name = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_GetConfigValueInfo(
                self.utils,
                value,
                &mut data_type,
                &mut scope,
            )
        };
        if name.is_null() {
            return None;
        }
        let name = unsafe { CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned();
        Some(NetworkingConfigInfo {
            name,
            data_type: data_type_from_sys(data_type)?,
            scope: scope_from_sys(scope),
        })
    }

    /// Returns the config value after `current` that a generic settings UI can edit. Pass `None` to start. Returns `None` at the end of the list.
    ///
    /// `include_dev_vars` also returns the development-only values, which a shipped build should not show to players.
    pub fn iterate_generic_editable_config_values(
        &self,
        current: Option<sys::ESteamNetworkingConfigValue>,
        include_dev_vars: bool,
    ) -> Option<sys::ESteamNetworkingConfigValue> {
        let current =
            current.unwrap_or(sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Invalid);
        let next = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_IterateGenericEditableConfigValues(
                self.utils,
                current,
                include_dev_vars,
            )
        };
        if next == sys::ESteamNetworkingConfigValue::k_ESteamNetworkingConfig_Invalid {
            None
        } else {
            Some(next)
        }
    }

    /// Formats an IP address as text, with the port when `with_port` is `true`. IPv6 addresses are in brackets when a port is shown.
    pub fn ip_addr_to_string(&self, addr: &NetworkingIpAddr, with_port: bool) -> String {
        // k_cchMaxString for SteamNetworkingIPAddr is 48; leave room for the port.
        let mut buffer = vec![0 as c_char; 64];
        unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SteamNetworkingIPAddr_ToString(
                self.utils,
                addr.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                with_port,
            );
        }
        c_buffer_to_string(&buffer)
    }

    /// Parses an IP address, with or without a port. Returns `None` when the text is not an address or contains a NUL byte.
    pub fn parse_ip_addr(&self, text: &str) -> Option<NetworkingIpAddr> {
        let text = CString::new(text).ok()?;
        let mut addr = NetworkingIpAddr::zeroed();
        let ok = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SteamNetworkingIPAddr_ParseString(
                self.utils,
                &mut addr.inner,
                text.as_ptr(),
            )
        };
        if ok {
            Some(addr)
        } else {
            None
        }
    }

    /// Classifies an IP address against Steam's fake IP range.
    pub fn ip_addr_fake_ip_type(&self, addr: &NetworkingIpAddr) -> FakeIpType {
        unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SteamNetworkingIPAddr_GetFakeIPType(
                self.utils,
                addr.as_ptr(),
            )
            .into()
        }
    }

    /// Formats a networking identity as text, such as a SteamID or an IP address. Use it to log or store an identity. Do not parse the text yourself.
    pub fn identity_to_string(&self, identity: &NetworkingIdentity) -> String {
        // k_cchMaxString for SteamNetworkingIdentity is 128.
        let mut buffer = vec![0 as c_char; 128];
        unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SteamNetworkingIdentity_ToString(
                self.utils,
                identity.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
            );
        }
        c_buffer_to_string(&buffer)
    }

    /// Parses text made by [`NetworkingUtils::identity_to_string`]. Returns `None` when the text is not understood or contains a NUL byte.
    pub fn parse_identity(&self, text: &str) -> Option<NetworkingIdentity> {
        let text = CString::new(text).ok()?;
        let mut identity = NetworkingIdentity::new();
        let ok = unsafe {
            sys::SteamAPI_ISteamNetworkingUtils_SteamNetworkingIdentity_ParseString(
                self.utils,
                identity.as_mut_ptr(),
                text.as_ptr(),
            )
        };
        if ok {
            Some(identity)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct RelayNetworkStatus {
    availability: NetworkingAvailabilityResult,
    is_ping_measurement_in_progress: bool,
    network_config: NetworkingAvailabilityResult,
    any_relay: NetworkingAvailabilityResult,

    debugging_message: String,
}

impl RelayNetworkStatus {
    /// Summary status.  When this is "current", initialization has
    /// completed.  Anything else means you are not ready yet, or
    /// there is a significant problem.
    pub fn availability(&self) -> NetworkingAvailabilityResult {
        self.availability.clone()
    }

    /// True if latency measurement is in progress (or pending, awaiting a prerequisite).
    pub fn is_ping_measurement_in_progress(&self) -> bool {
        self.is_ping_measurement_in_progress
    }

    /// Status obtaining the network config.  This is a prerequisite
    /// for relay network access.
    ///
    /// Failure to obtain the network config almost always indicates
    /// a problem with the local internet connection.
    pub fn network_config(&self) -> NetworkingAvailabilityResult {
        self.network_config.clone()
    }

    /// Current ability to communicate with ANY relay.  Note that
    /// the complete failure to communicate with any relays almost
    /// always indicates a problem with the local Internet connection.
    /// (However, just because you can reach a single relay doesn't
    /// mean that the local connection is in perfect health.)
    pub fn any_relay(&self) -> NetworkingAvailabilityResult {
        self.any_relay.clone()
    }

    /// Non-localized English language status.  For diagnostic/debugging
    /// purposes only.
    pub fn debugging_message(&self) -> &str {
        &self.debugging_message
    }
}

impl From<sys::SteamRelayNetworkStatus_t> for RelayNetworkStatus {
    fn from(status: steamworks_sys::SteamRelayNetworkStatus_t) -> Self {
        unsafe {
            Self {
                availability: status.m_eAvail.try_into(),
                is_ping_measurement_in_progress: status.m_bPingMeasurementInProgress != 0,
                network_config: status.m_eAvailNetworkConfig.try_into(),
                any_relay: status.m_eAvailAnyRelay.try_into(),
                debugging_message: CStr::from_ptr(status.m_debugMsg.as_ptr())
                    .to_str()
                    .expect("invalid debug string")
                    .to_owned(),
            }
        }
    }
}

#[derive(Debug)]
/// The relay network status callback.
pub struct RelayNetworkStatusCallback {
    status: RelayNetworkStatus,
}

impl_callback!(cb: SteamRelayNetworkStatus_t => RelayNetworkStatusCallback {
    Self {
        status: cb.into(),
    }
});

#[cfg(test)]
mod tests {
    use crate::Client;
    use std::time::Duration;

    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_networking_status() {
        let client = Client::init().unwrap();
        let callback_client = client.clone();
        std::thread::spawn(move || callback_client.run_callbacks());

        let utils = client.networking_utils();
        let status = utils.detailed_relay_network_status();
        println!(
            "status: {:?}, network_config: {:?}, any_relay: {:?}, message: {}",
            status.availability(),
            status.network_config(),
            status.any_relay(),
            status.debugging_message()
        );

        utils.init_relay_network_access();

        let status = utils.detailed_relay_network_status();
        println!(
            "status: {:?}, network_config: {:?}, any_relay: {:?}, message: {}",
            status.availability(),
            status.network_config(),
            status.any_relay(),
            status.debugging_message()
        );

        std::thread::sleep(Duration::from_millis(500));

        let status = utils.detailed_relay_network_status();
        println!(
            "status: {:?}, network_config: {:?}, any_relay: {:?}, message: {}",
            status.availability(),
            status.network_config(),
            status.any_relay(),
            status.debugging_message()
        );
    }
}
