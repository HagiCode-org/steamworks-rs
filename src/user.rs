use super::*;
use crate::networking_types::NetworkingIdentity;
#[cfg(test)]
use serial_test::serial;

/// Access to the steam user interface
pub struct User {
    pub(crate) user: *mut sys::ISteamUser,
    pub(crate) _inner: Arc<Inner>,
}

impl User {
    /// Returns the steam id of the current user
    pub fn steam_id(&self) -> SteamId {
        unsafe { SteamId(sys::SteamAPI_ISteamUser_GetSteamID(self.user)) }
    }

    /// Returns the level of the current user
    pub fn level(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamUser_GetPlayerSteamLevel(self.user) as u32 }
    }

    /// Returns whether the current user's Steam client is connected to the Steam servers.
    pub fn logged_on(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUser_BLoggedOn(self.user) }
    }

    /// Retrieve an authentication session ticket that can be sent
    /// to an entity that wishes to verify you.
    ///
    /// This ticket should not be reused.
    ///
    /// When creating ticket for use by the web API you should wait
    /// for the `AuthSessionTicketResponse` event before trying to
    /// use the ticket.
    ///
    /// When the multiplayer session terminates you must call
    /// `cancel_authentication_ticket`
    pub fn authentication_session_ticket_with_steam_id(
        &self,
        steam_id: SteamId,
    ) -> (AuthTicket, Vec<u8>) {
        self.authentication_session_ticket(NetworkingIdentity::new_steam_id(steam_id))
    }
    pub fn authentication_session_ticket(
        &self,
        network_identity: NetworkingIdentity,
    ) -> (AuthTicket, Vec<u8>) {
        unsafe {
            let mut ticket = vec![0; 1024];
            let mut ticket_len = 0;
            let auth_ticket = sys::SteamAPI_ISteamUser_GetAuthSessionTicket(
                self.user,
                ticket.as_mut_ptr().cast(),
                1024,
                &mut ticket_len,
                network_identity.as_ptr(),
            );
            ticket.truncate(ticket_len as usize);
            (AuthTicket(auth_ticket), ticket)
        }
    }

    /// Cancels an authentication session ticket received from
    /// `authentication_session_ticket`.
    ///
    /// This should be called when you are no longer playing with
    /// the specified entity.
    pub fn cancel_authentication_ticket(&self, ticket: AuthTicket) {
        unsafe {
            sys::SteamAPI_ISteamUser_CancelAuthTicket(self.user, ticket.0);
        }
    }

    /// Authenticate the ticket from the steam ID to make sure it is
    /// valid and not reused.
    ///
    /// A `ValidateAuthTicketResponse` callback will be fired if
    /// the entity goes offline or cancels the ticket.
    ///
    /// When the multiplayer session terminates you must call
    /// `end_authentication_session`
    pub fn begin_authentication_session(
        &self,
        user: SteamId,
        ticket: &[u8],
    ) -> Result<(), AuthSessionError> {
        unsafe {
            let res = sys::SteamAPI_ISteamUser_BeginAuthSession(
                self.user,
                ticket.as_ptr().cast(),
                ticket.len() as _,
                user.0,
            );
            Err(match res {
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultOK => return Ok(()),
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultInvalidTicket => {
                    AuthSessionError::InvalidTicket
                }
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultDuplicateRequest => {
                    AuthSessionError::DuplicateRequest
                }
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultInvalidVersion => {
                    AuthSessionError::InvalidVersion
                }
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultGameMismatch => {
                    AuthSessionError::GameMismatch
                }
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultExpiredTicket => {
                    AuthSessionError::ExpiredTicket
                }
                _ => unreachable!(),
            })
        }
    }

    /// Ends an authentication session that was started with
    /// `begin_authentication_session`.
    ///
    /// This should be called when you are no longer playing with
    /// the specified entity.
    pub fn end_authentication_session(&self, user: SteamId) {
        unsafe {
            sys::SteamAPI_ISteamUser_EndAuthSession(self.user, user.0);
        }
    }

    /// Retrieve an authentication ticket to be sent to the entity that
    /// wishes to authenticate you using the
    /// ISteamUserAuth/AuthenticateUserTicket Web API.
    ///
    /// The calling application must wait for the
    /// `TicketForWebApiResponse` callback generated  
    /// by the API call to access the ticket.
    ///  
    /// It is best practice to use an identity string for
    /// each service that will consume tickets.
    ///   
    /// This API can not be used to create a ticket for
    /// use by the BeginAuthSession/ISteamGameServer::BeginAuthSession.
    /// Use the `authentication_session_ticket` API instead
    pub fn authentication_session_ticket_for_webapi(&self, identity: &str) -> AuthTicket {
        unsafe {
            let c_str = CString::new(identity).unwrap();
            let auth_ticket =
                sys::SteamAPI_ISteamUser_GetAuthTicketForWebApi(self.user, c_str.as_ptr());

            AuthTicket(auth_ticket)
        }
    }

    /// Checks if the user owns a piece of DLC specified by app id.
    ///
    /// This can only be called after authenticating
    /// with the user using `begin_authentication_session`.
    pub fn user_has_license_for_app(&self, user: SteamId, app_id: AppId) -> UserHasLicense {
        unsafe {
            let license_response =
                sys::SteamAPI_ISteamUser_UserHasLicenseForApp(self.user, user.0, app_id.0);

            match license_response {
                sys::EUserHasLicenseForAppResult::k_EUserHasLicenseResultHasLicense => {
                    UserHasLicense::HasLicense
                }
                sys::EUserHasLicenseForAppResult::k_EUserHasLicenseResultDoesNotHaveLicense => {
                    UserHasLicense::DoesNotHaveLicense
                }
                sys::EUserHasLicenseForAppResult::k_EUserHasLicenseResultNoAuth => {
                    UserHasLicense::NoAuth
                }
                _ => unreachable!(),
            }
        }
    }
}

/// Errors from `begin_authentication_session`
#[derive(Debug, Error)]
pub enum AuthSessionError {
    /// The ticket is invalid
    #[error("invalid ticket")]
    InvalidTicket,
    /// A ticket has already been submitted for this steam ID
    #[error("duplicate ticket request")]
    DuplicateRequest,
    /// The ticket is from an incompatible interface version
    #[error("incompatible interface version")]
    InvalidVersion,
    /// The ticket is not for this game
    #[error("incorrect game for ticket")]
    GameMismatch,
    /// The ticket has expired
    #[error("ticket has expired")]
    ExpiredTicket,
}

#[test]
#[serial]
fn test_auth_dll() {
    let client = Client::init().unwrap();
    let user = client.user();

    let _cb = client.register_callback(|v: AuthSessionTicketResponse| {
        println!("Got dll auth response: {:?}", v)
    });
    let _cb = client.register_callback(|v: ValidateAuthTicketResponse| {
        println!("Got validate auth response: {:?}", v)
    });

    let id = user.steam_id();
    let (auth, ticket) = user.authentication_session_ticket_with_steam_id(id);

    println!("{:?}", auth);
    println!("{:?}", ticket);

    println!("{:?}", user.begin_authentication_session(id, &ticket));

    // this might still return NoAuth if the validation has not completed
    let has_space_war = user.user_has_license_for_app(id, AppId(480));
    println!("User has license response: {has_space_war:?}");

    for _ in 0..20 {
        client.run_callbacks();
        ::std::thread::sleep(::std::time::Duration::from_millis(50));
    }

    println!("END");

    user.cancel_authentication_ticket(auth);

    for _ in 0..20 {
        client.run_callbacks();
        ::std::thread::sleep(::std::time::Duration::from_millis(50));
    }

    user.end_authentication_session(id);
}

/// A handle for an authentication ticket that can be used to cancel
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AuthTicket(pub(crate) sys::HAuthTicket);

/// Called when generating a authentication session ticket.
///
/// This can be used to verify the ticket was created successfully.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AuthSessionTicketResponse {
    /// The ticket in question
    pub ticket: AuthTicket,
    /// The result of generating the ticket
    pub result: SResult<()>,
}

impl_callback!(cb: GetAuthSessionTicketResponse_t => AuthSessionTicketResponse {
    Self {
        ticket: AuthTicket(cb.m_hAuthTicket),
        result: crate::to_steam_result(cb.m_eResult),
    }
});

#[test]
#[serial]
fn test_auth_webapi() {
    let client = Client::init().unwrap();
    let user = client.user();

    let _cb = client.register_callback(|v: TicketForWebApiResponse| {
        println!("Got webapi auth response: {:?}", v)
    });

    let auth = user.authentication_session_ticket_for_webapi("myIdentity");

    println!("{:?}", auth);

    for _ in 0..20 {
        client.run_callbacks();
        ::std::thread::sleep(::std::time::Duration::from_millis(100));
    }

    println!("END");
}

/// Called when generating a authentication session ticket for web api.
///
/// This can be used to verify the ticket was created successfully.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TicketForWebApiResponse {
    pub ticket_handle: AuthTicket,
    pub result: SResult<()>,
    pub ticket_len: i32,
    pub ticket: Vec<u8>,
}

impl_callback!(cb: GetTicketForWebApiResponse_t => TicketForWebApiResponse {
    Self {
        ticket_handle: AuthTicket(cb.m_hAuthTicket),
        result: crate::to_steam_result(cb.m_eResult),
        ticket_len: cb.m_cubTicket,
        ticket: cb.m_rgubTicket.to_vec(),
    }
});

/// Called when an authentication ticket has been
/// validated.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ValidateAuthTicketResponse {
    /// The steam id of the entity that provided the ticket
    pub steam_id: SteamId,
    /// The result of the validation
    pub response: Result<(), AuthSessionValidateError>,
    /// The steam id of the owner of the game. Differs from
    /// `steam_id` if the game is borrowed.
    pub owner_steam_id: SteamId,
}

impl_callback!(cb: ValidateAuthTicketResponse_t => ValidateAuthTicketResponse {
    Self {
        steam_id: SteamId(cb.m_SteamID.m_steamid.m_unAll64Bits),
        owner_steam_id: SteamId(cb.m_OwnerSteamID.m_steamid.m_unAll64Bits),
        response: match cb.m_eAuthSessionResponse {
            sys::EAuthSessionResponse::k_EAuthSessionResponseOK => Ok(()),
            sys::EAuthSessionResponse::k_EAuthSessionResponseUserNotConnectedToSteam => {
                Err(AuthSessionValidateError::UserNotConnectedToSteam)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseNoLicenseOrExpired => {
                Err(AuthSessionValidateError::NoLicenseOrExpired)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseVACBanned => {
                Err(AuthSessionValidateError::VACBanned)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseLoggedInElseWhere => {
                Err(AuthSessionValidateError::LoggedInElseWhere)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseVACCheckTimedOut => {
                Err(AuthSessionValidateError::VACCheckTimedOut)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseAuthTicketCanceled => {
                Err(AuthSessionValidateError::AuthTicketCancelled)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseAuthTicketInvalidAlreadyUsed => {
                Err(AuthSessionValidateError::AuthTicketInvalidAlreadyUsed)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseAuthTicketInvalid => {
                Err(AuthSessionValidateError::AuthTicketInvalid)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponsePublisherIssuedBan => {
                Err(AuthSessionValidateError::PublisherIssuedBan)
            }
            _ => unreachable!(),
        },
    }
});

// Called when a microtransaction authorization response is received
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MicroTxnAuthorizationResponse {
    pub app_id: AppId,
    pub order_id: u64,
    pub authorized: bool,
}

impl_callback!(cb: MicroTxnAuthorizationResponse_t => MicroTxnAuthorizationResponse {
    Self {
        app_id: cb.m_unAppID.into(),
        order_id: cb.m_ulOrderID.into(),
        authorized: cb.m_bAuthorized == 1,
    }
});

/// Called when a connection to the Steam servers is made.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamServersConnected;

impl_callback!(_cb: SteamServersConnected_t => SteamServersConnected {
    Self
});

/// Called when the connection to the Steam servers is lost.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamServersDisconnected {
    /// The reason we were disconnected from the Steam servers
    pub reason: SteamError,
}

impl_callback!(cb: SteamServersDisconnected_t => SteamServersDisconnected {
    Self {
        reason: cb.m_eResult.into(),
    }
});

/// Called when the connection to the Steam servers fails.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamServerConnectFailure {
    /// The reason we failed to connect to the Steam servers
    pub reason: SteamError,
    /// Whether we are still retrying the connection.
    pub still_retrying: bool,
}

impl_callback!(cb: SteamServerConnectFailure_t => SteamServerConnectFailure {
    Self {
        reason: cb.m_eResult.into(),
        still_retrying: cb.m_bStillRetrying,
    }
});

/// Errors from `ValidateAuthTicketResponse`
#[derive(Clone, Debug, Error)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AuthSessionValidateError {
    /// The user in question is not connected to steam
    #[error("user not connected to steam")]
    UserNotConnectedToSteam,
    /// The license has expired
    #[error("the license has expired")]
    NoLicenseOrExpired,
    /// The user is VAC banned from the game
    #[error("the user is VAC banned from this game")]
    VACBanned,
    /// The user has logged in elsewhere and the session
    /// has been disconnected
    #[error("the user is logged in elsewhere")]
    LoggedInElseWhere,
    /// VAC has been unable to perform anti-cheat checks on this
    /// user
    #[error("VAC check timed out")]
    VACCheckTimedOut,
    /// The ticket has been cancelled by the issuer
    #[error("the authentication ticket has been cancelled")]
    AuthTicketCancelled,
    /// The ticket has already been used
    #[error("the authentication ticket has already been used")]
    AuthTicketInvalidAlreadyUsed,
    /// The ticket is not from a user instance currently connected
    /// to steam
    #[error("the authentication ticket is invalid")]
    AuthTicketInvalid,
    /// The user is banned from the game (not VAC)
    #[error("the user is banned")]
    PublisherIssuedBan,
}

/// Results from [`User::user_has_license_for_app`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserHasLicense {
    /// The user has a license for the specified app.
    HasLicense,
    /// The user does not have a license for the specified app.
    DoesNotHaveLicense,
    /// The user has not been authenticated.
    NoAuth,
}

/// Result of a voice call, from the SDK's `EVoiceResult`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VoiceResult {
    /// The call succeeded.
    Ok,
    /// Voice is not initialized.
    NotInitialized,
    /// Voice is not recording. `get_voice` returns this once a stopped recording has no audio left.
    NotRecording,
    /// No captured audio is available.
    NoData,
    /// The destination buffer is too small. See `decompress_voice` for the size that is needed.
    BufferTooSmall,
    /// The data is corrupted.
    DataCorrupted,
    /// Voice is restricted for this user.
    Restricted,
    /// The codec is not supported.
    UnsupportedCodec,
    /// The receiver runs an out-of-date version.
    ReceiverOutOfDate,
    /// The receiver did not answer.
    ReceiverDidNotAnswer,
    /// A value this binding does not name. The SDK enum is `#[non_exhaustive]`, so a newer SDK can add values.
    Unknown,
}

impl From<sys::EVoiceResult> for VoiceResult {
    fn from(result: sys::EVoiceResult) -> Self {
        match result {
            sys::EVoiceResult::k_EVoiceResultOK => VoiceResult::Ok,
            sys::EVoiceResult::k_EVoiceResultNotInitialized => VoiceResult::NotInitialized,
            sys::EVoiceResult::k_EVoiceResultNotRecording => VoiceResult::NotRecording,
            sys::EVoiceResult::k_EVoiceResultNoData => VoiceResult::NoData,
            sys::EVoiceResult::k_EVoiceResultBufferTooSmall => VoiceResult::BufferTooSmall,
            sys::EVoiceResult::k_EVoiceResultDataCorrupted => VoiceResult::DataCorrupted,
            sys::EVoiceResult::k_EVoiceResultRestricted => VoiceResult::Restricted,
            sys::EVoiceResult::k_EVoiceResultUnsupportedCodec => VoiceResult::UnsupportedCodec,
            sys::EVoiceResult::k_EVoiceResultReceiverOutOfDate => VoiceResult::ReceiverOutOfDate,
            sys::EVoiceResult::k_EVoiceResultReceiverDidNotAnswer => {
                VoiceResult::ReceiverDidNotAnswer
            }
            _ => VoiceResult::Unknown,
        }
    }
}

bitflags! {
    /// Reasons a user may not use the Steam Community Market, from the SDK's
    /// `EMarketNotAllowedReasonFlags`. Used in [`MarketEligibility`].
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct MarketNotAllowedReasons: u32 {
        /// A back-end call failed. Retrying may work.
        const TEMPORARY_FAILURE = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_TemporaryFailure.0 as _;
        /// The account is disabled.
        const ACCOUNT_DISABLED = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_AccountDisabled.0 as _;
        /// The account is locked down.
        const ACCOUNT_LOCKED_DOWN = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_AccountLockedDown.0 as _;
        /// The account is limited (no purchases).
        const ACCOUNT_LIMITED = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_AccountLimited.0 as _;
        /// The account is banned from trading items.
        const TRADE_BANNED = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_TradeBanned.0 as _;
        /// The account is not trusted.
        const ACCOUNT_NOT_TRUSTED = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_AccountNotTrusted.0 as _;
        /// Steam Guard is not enabled.
        const STEAM_GUARD_NOT_ENABLED = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_SteamGuardNotEnabled.0 as _;
        /// Steam Guard was enabled only recently.
        const STEAM_GUARD_ONLY_RECENTLY_ENABLED = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_SteamGuardOnlyRecentlyEnabled.0 as _;
        /// The password was reset recently.
        const RECENT_PASSWORD_RESET = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_RecentPasswordReset.0 as _;
        /// A new payment method was added recently.
        const NEW_PAYMENT_METHOD = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_NewPaymentMethod.0 as _;
        /// An invalid cookie was sent.
        const INVALID_COOKIE = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_InvalidCookie.0 as _;
        /// The user is on a new device or browser.
        const USING_NEW_DEVICE = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_UsingNewDevice.0 as _;
        /// A recent self-refund happened.
        const RECENT_SELF_REFUND = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_RecentSelfRefund.0 as _;
        /// The new payment method cannot be verified.
        const NEW_PAYMENT_METHOD_CANNOT_BE_VERIFIED = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_NewPaymentMethodCannotBeVerified.0 as _;
        /// There are no recent purchases.
        const NO_RECENT_PURCHASES = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_NoRecentPurchases.0 as _;
        /// A wallet gift was accepted recently.
        const ACCEPTED_WALLET_GIFT = sys::EMarketNotAllowedReasonFlags::k_EMarketNotAllowedReason_AcceptedWalletGift.0 as _;
    }
}

/// The Community Market status of the current user, from `MarketEligibilityResponse_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MarketEligibility {
    /// Whether the user may use the Community Market.
    pub allowed: bool,
    /// Why the user may not use it. Empty when `allowed` is true.
    pub not_allowed_reasons: MarketNotAllowedReasons,
    /// The time from which the user may use the Market, as the SDK's `RTime32`. The header names only the type, so the epoch is unverified.
    pub allowed_at_time: u32,
    /// Days of Steam Guard the user must have had before using the Market.
    pub steam_guard_required_days: i32,
    /// Days after first authorizing a device before the user may use the Market on that device.
    pub new_device_cooldown_days: i32,
}

/// Playtime progress for the duration control feature, from the SDK's `EDurationControlProgress`.
///
/// `Half` and `NoProgress` are deprecated in the SDK.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DurationControlProgress {
    /// Everything is fine: full progress.
    Full,
    /// Deprecated: XP or persistent rewards should be halved.
    Half,
    /// Deprecated: XP or persistent rewards should be stopped.
    NoProgress,
    /// The allowed 3 hours since the 5-hour gap have passed; the game should exit.
    ExitSoon3h,
    /// The allowed 5 hours in the calendar day have passed; the game should exit.
    ExitSoon5h,
    /// The game runs after the day period; it should exit.
    ExitSoonNight,
    /// A value this binding does not name. The SDK enum is `#[non_exhaustive]`.
    Unknown,
}

impl From<sys::EDurationControlProgress> for DurationControlProgress {
    fn from(progress: sys::EDurationControlProgress) -> Self {
        match progress {
            sys::EDurationControlProgress::k_EDurationControlProgress_Full => {
                DurationControlProgress::Full
            }
            sys::EDurationControlProgress::k_EDurationControlProgress_Half => {
                DurationControlProgress::Half
            }
            sys::EDurationControlProgress::k_EDurationControlProgress_None => {
                DurationControlProgress::NoProgress
            }
            sys::EDurationControlProgress::k_EDurationControl_ExitSoon_3h => {
                DurationControlProgress::ExitSoon3h
            }
            sys::EDurationControlProgress::k_EDurationControl_ExitSoon_5h => {
                DurationControlProgress::ExitSoon5h
            }
            sys::EDurationControlProgress::k_EDurationControl_ExitSoon_Night => {
                DurationControlProgress::ExitSoonNight
            }
            _ => DurationControlProgress::Unknown,
        }
    }
}

/// Which duration control notification timer expired, from the SDK's `EDurationControlNotification`.
///
/// `ThreeHours`, `HalfProgress` and `NoProgress` are deprecated in the SDK.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DurationControlNotification {
    /// Progress information only; there is nothing to show.
    NoNotification,
    /// The user has been playing for an hour; the value is a progress report.
    OneHour,
    /// Deprecated: the user has been playing for 3 hours.
    ThreeHours,
    /// Deprecated: progress is half normal.
    HalfProgress,
    /// Deprecated: progress is zero.
    NoProgress,
    /// The allowed 3 hours since the 5-hour gap have passed; the game should exit.
    ExitSoon3h,
    /// The allowed 5 hours in the calendar day have passed; the game should exit.
    ExitSoon5h,
    /// The game runs after the day period; it should exit.
    ExitSoonNight,
    /// A value this binding does not name. The SDK enum is `#[non_exhaustive]`.
    Unknown,
}

impl From<sys::EDurationControlNotification> for DurationControlNotification {
    fn from(notification: sys::EDurationControlNotification) -> Self {
        match notification {
            sys::EDurationControlNotification::k_EDurationControlNotification_None => {
                DurationControlNotification::NoNotification
            }
            sys::EDurationControlNotification::k_EDurationControlNotification_1Hour => {
                DurationControlNotification::OneHour
            }
            sys::EDurationControlNotification::k_EDurationControlNotification_3Hours => {
                DurationControlNotification::ThreeHours
            }
            sys::EDurationControlNotification::k_EDurationControlNotification_HalfProgress => {
                DurationControlNotification::HalfProgress
            }
            sys::EDurationControlNotification::k_EDurationControlNotification_NoProgress => {
                DurationControlNotification::NoProgress
            }
            sys::EDurationControlNotification::k_EDurationControlNotification_ExitSoon_3h => {
                DurationControlNotification::ExitSoon3h
            }
            sys::EDurationControlNotification::k_EDurationControlNotification_ExitSoon_5h => {
                DurationControlNotification::ExitSoon5h
            }
            sys::EDurationControlNotification::k_EDurationControlNotification_ExitSoon_Night => {
                DurationControlNotification::ExitSoonNight
            }
            _ => DurationControlNotification::Unknown,
        }
    }
}

/// The online state of the game for duration control, from the SDK's `EDurationControlOnlineState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DurationControlOnlineState {
    /// Offline play: single player, offline co-op and the like. Offline time does not count against playtime limits.
    Offline,
    /// Currently in online play.
    Online,
    /// Currently in online play and asking not to be interrupted.
    OnlineHighPri,
}

impl From<DurationControlOnlineState> for sys::EDurationControlOnlineState {
    fn from(state: DurationControlOnlineState) -> Self {
        match state {
            DurationControlOnlineState::Offline => {
                sys::EDurationControlOnlineState::k_EDurationControlOnlineState_Offline
            }
            DurationControlOnlineState::Online => {
                sys::EDurationControlOnlineState::k_EDurationControlOnlineState_Online
            }
            DurationControlOnlineState::OnlineHighPri => {
                sys::EDurationControlOnlineState::k_EDurationControlOnlineState_OnlineHighPri
            }
        }
    }
}

/// The duration control (playtime limit) status, from `DurationControl_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DurationControl {
    /// The app that generated the playtime.
    pub app_id: AppId,
    /// Whether duration control applies to this user and game.
    pub applicable: bool,
    /// Playtime since the most recent 5-hour gap, in seconds, counted up to the regulatory limit.
    pub seconds_last_5h: i32,
    /// The recommended progress.
    pub progress: DurationControlProgress,
    /// The notification to show, if any. Always `NoNotification` for call results.
    pub notification: DurationControlNotification,
    /// Playtime on the current calendar day, in seconds.
    pub seconds_today: i32,
    /// Playtime left before the user reaches a regulatory limit, in seconds.
    pub seconds_remaining: i32,
}

/// Returns the bytes of a fixed-size C char array up to its first NUL, decoded as UTF-8 with replacement.
pub(crate) fn c_chars_to_string(chars: &[c_char]) -> String {
    let bytes: Vec<u8> = chars
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

impl User {
    /// Returns the raw `HSteamUser` handle that this interface represents.
    ///
    /// The SDK header says the API uses it internally, and a few interfaces that support several users take it as input.
    /// Most code does not need it.
    pub fn hsteam_user(&self) -> sys::HSteamUser {
        unsafe { sys::SteamAPI_ISteamUser_GetHSteamUser(self.user) }
    }

    /// Reports a legacy usage event to the Steam client for a game. Needs `init()`.
    ///
    /// `game_id` is the raw `CGameID`; for a game without mods it is the app id. The SDK header does not list the event numbers,
    /// so `event` is passed through unchanged and the mapping is unverified against a live Steam client.
    /// Pass an empty `extra_info` when there is nothing extra to send.
    ///
    /// This sends data to Steam. Returns `Err(())` if `extra_info` contains a NUL byte, and then nothing is sent.
    pub fn track_app_usage_event(
        &self,
        game_id: GameId,
        event: i32,
        extra_info: &str,
    ) -> Result<(), ()> {
        let extra_info = CString::new(extra_info).map_err(|_| ())?;
        unsafe {
            sys::SteamAPI_ISteamUser_TrackAppUsageEvent(
                self.user,
                game_id.0,
                event,
                extra_info.as_ptr(),
            );
        }
        Ok(())
    }

    /// Returns the local folder that Steam provides for the current account's data for this app, such as save games and configs.
    ///
    /// The header gives the form `...\userdata\<SteamID>\<AppID>\local`. Returns `None` when the SDK reports failure, or when the path
    /// does not end inside the 4096-byte buffer. Invalid UTF-8 is replaced.
    pub fn user_data_folder(&self) -> Option<String> {
        const BUFFER_LEN: usize = 4096;
        let mut buffer = vec![0u8; BUFFER_LEN];
        let ok = unsafe {
            sys::SteamAPI_ISteamUser_GetUserDataFolder(
                self.user,
                buffer.as_mut_ptr().cast(),
                BUFFER_LEN as _,
            )
        };
        if !ok {
            return None;
        }
        let end = buffer.iter().position(|b| *b == 0)?;
        Some(String::from_utf8_lossy(&buffer[..end]).into_owned())
    }

    /// Starts recording voice from the microphone. Read the captured audio with `get_voice`, at least once per frame.
    pub fn start_voice_recording(&self) {
        unsafe { sys::SteamAPI_ISteamUser_StartVoiceRecording(self.user) }
    }

    /// Stops recording. Steam keeps recording for a short time after the call, so keep calling `get_voice` until it returns
    /// `VoiceResult::NotRecording`.
    pub fn stop_voice_recording(&self) {
        unsafe { sys::SteamAPI_ISteamUser_StopVoiceRecording(self.user) }
    }

    /// Returns how many bytes of compressed voice data `get_voice` can return now. Only the compressed size is requested;
    /// the deprecated uncompressed parameters are never used.
    ///
    /// Returns `Err` with the non-OK result when Steam reports one.
    pub fn available_voice(&self) -> Result<u32, VoiceResult> {
        let mut compressed = 0u32;
        let result = unsafe {
            sys::SteamAPI_ISteamUser_GetAvailableVoice(
                self.user,
                &mut compressed,
                std::ptr::null_mut(),
                0,
            )
        };
        match VoiceResult::from(result) {
            VoiceResult::Ok => Ok(compressed),
            other => Err(other),
        }
    }

    /// Copies captured compressed voice data into `buffer`. Returns the result and the number of bytes written.
    ///
    /// Decode the data with `decompress_voice`. Returns `Err(())` if `buffer` is longer than `u32::MAX` bytes.
    pub fn get_voice(&self, buffer: &mut [u8]) -> Result<(VoiceResult, usize), ()> {
        let len = u32::try_from(buffer.len()).map_err(|_| ())?;
        let mut written = 0u32;
        let result = unsafe {
            sys::SteamAPI_ISteamUser_GetVoice(
                self.user,
                true,
                buffer.as_mut_ptr().cast(),
                len,
                &mut written,
                false,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0,
            )
        };
        Ok((VoiceResult::from(result), written as usize))
    }

    /// Decodes compressed voice data from `get_voice` into raw single-channel 16-bit PCM in `buffer`.
    ///
    /// `desired_sample_rate` should be in the 11025 to 48000 range the header lists; `voice_optimal_sample_rate` returns the
    /// native rate. Returns the result and the bytes written. For `VoiceResult::BufferTooSmall` the count is the size needed,
    /// and the call should be repeated with a bigger buffer. Returns `Err(())` if either length does not fit in `u32`.
    pub fn decompress_voice(
        &self,
        compressed: &[u8],
        desired_sample_rate: u32,
        buffer: &mut [u8],
    ) -> Result<(VoiceResult, usize), ()> {
        let compressed_len = u32::try_from(compressed.len()).map_err(|_| ())?;
        let buffer_len = u32::try_from(buffer.len()).map_err(|_| ())?;
        let mut written = 0u32;
        let result = unsafe {
            sys::SteamAPI_ISteamUser_DecompressVoice(
                self.user,
                compressed.as_ptr().cast(),
                compressed_len,
                buffer.as_mut_ptr().cast(),
                buffer_len,
                &mut written,
                desired_sample_rate,
            )
        };
        Ok((VoiceResult::from(result), written as usize))
    }

    /// Returns the native sample rate of the Steam voice decompressor. Decoding at this rate does the least work;
    /// the rate of your audio output device, usually 44100 or 48000, can give better output quality.
    pub fn voice_optimal_sample_rate(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamUser_GetVoiceOptimalSampleRate(self.user) }
    }

    /// Returns whether Steam thinks the user is behind a NAT device.
    ///
    /// Only valid after the client has connected to Steam (a `SteamServersConnected` callback has fired). It may not detect every form of NAT.
    pub fn is_behind_nat(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUser_BIsBehindNAT(self.user) }
    }

    /// Publishes a game server's address so friends can join this game. Steam replicates the data to friends.
    ///
    /// This sends data to Steam. `ip` is an IPv4 address as a `u32`; the header does not say which byte order the SDK expects,
    /// so the value is passed through unchanged (unverified against a live Steam client).
    pub fn advertise_game(&self, game_server: SteamId, ip: u32, port: u16) {
        unsafe { sys::SteamAPI_ISteamUser_AdvertiseGame(self.user, game_server.0, ip, port) }
    }

    /// Requests an encrypted app ticket, optionally with `data` encrypted into it.
    ///
    /// `cb` runs with `Ok(())` once Steam has the ticket, and the ticket is then read with `encrypted_app_ticket`.
    /// The result arrives through `run_callbacks()`. This sends a request to Steam.
    /// Returns `Err(())` if `data` is longer than `i32::MAX` bytes; nothing is sent then.
    pub fn request_encrypted_app_ticket(
        &self,
        data: Option<&[u8]>,
        cb: impl FnOnce(Result<(), SteamError>) + 'static + Send,
    ) -> Result<(), ()> {
        // The copy lives inside the callback, so it is valid until the result arrives.
        let mut owned = data.unwrap_or(&[]).to_vec();
        let len = i32::try_from(owned.len()).map_err(|_| ())?;
        let api_call = unsafe {
            sys::SteamAPI_ISteamUser_RequestEncryptedAppTicket(
                self.user,
                owned.as_mut_ptr().cast(),
                len,
            )
        };
        unsafe {
            register_call_result::<sys::EncryptedAppTicketResponse_t, _>(
                &self._inner,
                api_call,
                move |v, io_error| {
                    let _owned = &owned;
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else if v.m_eResult == sys::EResult::k_EResultOK {
                        Ok(())
                    } else {
                        Err(v.m_eResult.into())
                    })
                },
            );
        }
        Ok(())
    }

    /// Returns the encrypted app ticket that `request_encrypted_app_ticket` asked for, or `None` if no ticket is ready.
    ///
    /// The SDK reports the size first, so the ticket is fetched with a buffer of that size. Returns `None` when the size is zero,
    /// the size does not fit in `i32`, or the SDK refuses the copy.
    pub fn encrypted_app_ticket(&self) -> Option<Vec<u8>> {
        let mut size = 0u32;
        unsafe {
            sys::SteamAPI_ISteamUser_GetEncryptedAppTicket(
                self.user,
                std::ptr::null_mut(),
                0,
                &mut size,
            );
        }
        if size == 0 {
            return None;
        }
        let max = i32::try_from(size).ok()?;
        let mut ticket = vec![0u8; size as usize];
        let mut written = 0u32;
        let ok = unsafe {
            sys::SteamAPI_ISteamUser_GetEncryptedAppTicket(
                self.user,
                ticket.as_mut_ptr().cast(),
                max,
                &mut written,
            )
        };
        if !ok {
            return None;
        }
        ticket.truncate(written as usize);
        Some(ticket)
    }

    /// Returns the user's trading-card badge level for a series.
    ///
    /// `series` is the card series number. `foil` selects the foil badge, which has a maximum level of 1; the regular badge
    /// maximum is 5 (both from the header). The header does not say which value means "no badge", so the value is passed through unverified.
    pub fn game_badge_level(&self, series: i32, foil: bool) -> i32 {
        unsafe { sys::SteamAPI_ISteamUser_GetGameBadgeLevel(self.user, series, foil) }
    }

    /// Requests a store checkout URL that signs in the in-game browser, then redirects to `redirect_url`.
    ///
    /// `cb` receives the URL. The SDK says the URL has a very short lifetime, so open it straight away; its auth cookie lasts a day,
    /// so the SDK suggests requesting a new URL about every 12 hours. The result arrives through `run_callbacks()`.
    /// Returns `Err(())` if `redirect_url` contains a NUL byte. `cb` gets `Err(SteamError::Generic)` when Steam returns an empty URL.
    pub fn request_store_auth_url(
        &self,
        redirect_url: &str,
        cb: impl FnOnce(Result<String, SteamError>) + 'static + Send,
    ) -> Result<(), ()> {
        let redirect_url = CString::new(redirect_url).map_err(|_| ())?;
        let api_call = unsafe {
            sys::SteamAPI_ISteamUser_RequestStoreAuthURL(self.user, redirect_url.as_ptr())
        };
        unsafe {
            register_call_result::<sys::StoreAuthURLResponse_t, _>(
                &self._inner,
                api_call,
                move |v, io_error| {
                    if io_error {
                        cb(Err(SteamError::IOFailure));
                        return;
                    }
                    let url = c_chars_to_string(&v.m_szURL);
                    cb(if url.is_empty() {
                        Err(SteamError::Generic)
                    } else {
                        Ok(url)
                    })
                },
            );
        }
        Ok(())
    }

    /// Returns whether the user's phone number is verified.
    pub fn is_phone_verified(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUser_BIsPhoneVerified(self.user) }
    }

    /// Returns whether the user has two-factor authentication enabled on the account.
    pub fn is_two_factor_enabled(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUser_BIsTwoFactorEnabled(self.user) }
    }

    /// Returns whether the user's phone number is an identifying one.
    pub fn is_phone_identifying(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUser_BIsPhoneIdentifying(self.user) }
    }

    /// Returns whether the user's phone number is waiting for (re)verification.
    pub fn is_phone_requiring_verification(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUser_BIsPhoneRequiringVerification(self.user) }
    }

    /// Requests whether the current user may use the Community Market. `cb` receives the answer; the result arrives through `run_callbacks()`.
    ///
    /// `cb` gets `Err(SteamError::IOFailure)` when the call did not complete. The SDK call result has no error code of its own.
    pub fn get_market_eligibility(
        &self,
        cb: impl FnOnce(Result<MarketEligibility, SteamError>) + 'static + Send,
    ) {
        let api_call = unsafe { sys::SteamAPI_ISteamUser_GetMarketEligibility(self.user) };
        unsafe {
            register_call_result::<sys::MarketEligibilityResponse_t, _>(
                &self._inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else {
                        Ok(MarketEligibility {
                            allowed: v.m_bAllowed,
                            not_allowed_reasons: MarketNotAllowedReasons::from_bits_retain(
                                v.m_eNotAllowedReason.0 as u32,
                            ),
                            allowed_at_time: v.m_rtAllowedAtTime,
                            steam_guard_required_days: v.m_cdaySteamGuardRequiredDays,
                            new_device_cooldown_days: v.m_cdayNewDeviceCooldown,
                        })
                    })
                },
            );
        }
    }

    /// Requests the duration control (anti-indulgence playtime limit) status of the current user. `cb` receives the status;
    /// the result arrives through `run_callbacks()`.
    ///
    /// `cb` gets the SDK's error result as an `Err` when it is not OK, and `Err(SteamError::IOFailure)` when the call did not complete.
    pub fn get_duration_control(
        &self,
        cb: impl FnOnce(Result<DurationControl, SteamError>) + 'static + Send,
    ) {
        let api_call = unsafe { sys::SteamAPI_ISteamUser_GetDurationControl(self.user) };
        unsafe {
            register_call_result::<sys::DurationControl_t, _>(
                &self._inner,
                api_call,
                move |v, io_error| {
                    if io_error {
                        cb(Err(SteamError::IOFailure));
                        return;
                    }
                    if v.m_eResult != sys::EResult::k_EResultOK {
                        cb(Err(v.m_eResult.into()));
                        return;
                    }
                    cb(Ok(DurationControl {
                        app_id: AppId(v.m_appid),
                        applicable: v.m_bApplicable,
                        seconds_last_5h: v.m_csecsLast5h,
                        progress: v.m_progress.into(),
                        notification: v.m_notification.into(),
                        seconds_today: v.m_csecsToday,
                        seconds_remaining: v.m_csecsRemaining,
                    }))
                },
            );
        }
    }

    /// Tells the SDK's duration control system whether the game is online. Offline play does not count against playtime limits.
    ///
    /// This sends the state to Steam. Returns whether the SDK accepted the state.
    pub fn set_duration_control_online_state(&self, state: DurationControlOnlineState) -> bool {
        unsafe { sys::SteamAPI_ISteamUser_BSetDurationControlOnlineState(self.user, state.into()) }
    }
}
