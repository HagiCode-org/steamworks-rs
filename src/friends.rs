use super::*;
use std::net::Ipv4Addr;

bitflags! {
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[repr(C)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct FriendFlags: u16 {
        const NONE                  = 0x0000;
        const BLOCKED               = 0x0001;
        const FRIENDSHIP_REQUESTED  = 0x0002;
        const IMMEDIATE             = 0x0004;
        const CLAN_MEMBER           = 0x0008;
        const ON_GAME_SERVER        = 0x0010;
        // Unused
        // Unused
        const REQUESTING_FRIENDSHIP = 0x0080;
        const REQUESTING_INFO       = 0x0100;
        const IGNORED               = 0x0200;
        const IGNORED_FRIEND        = 0x0400;
        // Unused
        const CHAT_MEMBER           = 0x1000;
        const ALL                   = 0xFFFF;
    }
}

bitflags! {
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[repr(C)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct PersonaChange: i32 {
        const NAME                = 0x0001;
        const STATUS              = 0x0002;
        const COME_ONLINE         = 0x0004;
        const GONE_OFFLINE        = 0x0008;
        const GAME_PLAYED         = 0x0010;
        const GAME_SERVER         = 0x0020;
        const AVATAR              = 0x0040;
        const JOINED_SOURCE       = 0x0080;
        const LEFT_SOURCE         = 0x0100;
        const RELATIONSHIP_CHANGE = 0x0200;
        const NAME_FIRST_SET      = 0x0400;
        const FACEBOOK_INFO       = 0x0800;
        const NICKNAME            = 0x1000;
        const STEAM_LEVEL         = 0x2000;
    }
}

pub enum OverlayToStoreFlag {
    None = 0,
    AddToCart = 1,
    AddToCartAndShow = 2,
}

/// Access to the steam friends interface
pub struct Friends {
    pub(crate) friends: *mut sys::ISteamFriends,
    pub(crate) inner: Arc<Inner>,
}

impl Friends {
    /// Returns the (display) name of the current user
    pub fn name(&self) -> String {
        unsafe {
            let name = sys::SteamAPI_ISteamFriends_GetPersonaName(self.friends);
            let name = CStr::from_ptr(name);
            name.to_string_lossy().into_owned()
        }
    }

    pub fn get_friends(&self, flags: FriendFlags) -> Vec<Friend> {
        unsafe {
            let count = sys::SteamAPI_ISteamFriends_GetFriendCount(self.friends, flags.bits() as _);
            if count == -1 {
                return Vec::new();
            }
            let mut friends = Vec::with_capacity(count as usize);
            for idx in 0..count {
                let friend = SteamId(sys::SteamAPI_ISteamFriends_GetFriendByIndex(
                    self.friends,
                    idx,
                    flags.bits() as _,
                ));
                friends.push(self.get_friend(friend));
            }

            friends
        }
    }
    /// Returns recently played with players list
    pub fn get_coplay_friends(&self) -> Vec<Friend> {
        unsafe {
            let count = sys::SteamAPI_ISteamFriends_GetCoplayFriendCount(self.friends);
            if count == -1 {
                return Vec::new();
            }
            let mut friends = Vec::with_capacity(count as usize);
            for idx in 0..count {
                let friend = SteamId(sys::SteamAPI_ISteamFriends_GetCoplayFriend(
                    self.friends,
                    idx,
                ));
                friends.push(self.get_friend(friend));
            }
            friends
        }
    }

    pub fn get_friend(&self, friend: SteamId) -> Friend {
        Friend {
            id: friend,
            friends: self.friends,
            _inner: self.inner.clone(),
        }
    }

    pub fn request_user_information(&self, user: SteamId, name_only: bool) -> bool {
        unsafe {
            sys::SteamAPI_ISteamFriends_RequestUserInformation(self.friends, user.0, name_only)
        }
    }

    pub fn activate_game_overlay(&self, dialog: &str) {
        let dialog = CString::new(dialog).unwrap();
        unsafe {
            sys::SteamAPI_ISteamFriends_ActivateGameOverlay(self.friends, dialog.as_ptr());
        }
    }

    // I don't know why these are part of friends either
    pub fn activate_game_overlay_to_web_page(&self, url: &str) {
        unsafe {
            let url = CString::new(url).unwrap();
            sys::SteamAPI_ISteamFriends_ActivateGameOverlayToWebPage(
                self.friends,
                url.as_ptr(),
                sys::EActivateGameOverlayToWebPageMode::k_EActivateGameOverlayToWebPageMode_Default,
            );
        }
    }

    pub fn activate_game_overlay_to_store(
        &self,
        app_id: AppId,
        overlay_to_store_flag: OverlayToStoreFlag,
    ) {
        unsafe {
            let overlay_to_store_flag = match overlay_to_store_flag {
                OverlayToStoreFlag::None => sys::EOverlayToStoreFlag::k_EOverlayToStoreFlag_None,
                OverlayToStoreFlag::AddToCart => {
                    sys::EOverlayToStoreFlag::k_EOverlayToStoreFlag_AddToCart
                }
                OverlayToStoreFlag::AddToCartAndShow => {
                    sys::EOverlayToStoreFlag::k_EOverlayToStoreFlag_AddToCartAndShow
                }
            };
            sys::SteamAPI_ISteamFriends_ActivateGameOverlayToStore(
                self.friends,
                app_id.0,
                overlay_to_store_flag,
            );
        }
    }

    pub fn activate_game_overlay_to_user(&self, dialog: &str, user: SteamId) {
        let dialog = CString::new(dialog).unwrap();
        unsafe {
            sys::SteamAPI_ISteamFriends_ActivateGameOverlayToUser(
                self.friends,
                dialog.as_ptr(),
                user.0,
            );
        }
    }

    /// Opens up an invite dialog for the given lobby
    pub fn activate_invite_dialog(&self, lobby: LobbyId) {
        unsafe {
            sys::SteamAPI_ISteamFriends_ActivateGameOverlayInviteDialog(self.friends, lobby.0);
        }
    }

    /// Opens up an invite dialog that will send Rich Presence connect string to friends
    ///
    /// # Panics
    ///
    /// Panics if the `connect` str contains a null byte.
    pub fn activate_invite_dialog_connect_string(&self, connect: &str) {
        let connect = CString::new(connect).unwrap();
        unsafe {
            sys::SteamAPI_ISteamFriends_ActivateGameOverlayInviteDialogConnectString(
                self.friends,
                connect.as_ptr(),
            );
        }
    }

    /// Set rich presence for the user. Unsets the rich presence if `value` is None or empty.
    ///
    /// See [Steam API](https://partner.steamgames.com/doc/api/ISteamFriends#SetRichPresence)
    ///
    /// # Panics
    ///
    /// Panics if the `key` or `value` str slices contain a null byte.
    pub fn set_rich_presence(&self, key: &str, value: Option<&str>) -> bool {
        let key = CString::new(key).unwrap();
        let value = value.map(|v| CString::new(v).unwrap());
        let value_ptr = value
            .as_ref()
            .map_or(std::ptr::null(), |value| value.as_ptr());
        unsafe {
            sys::SteamAPI_ISteamFriends_SetRichPresence(self.friends, key.as_ptr(), value_ptr)
        }
    }

    /// Clears all of the current user's Rich Presence key/values.
    pub fn clear_rich_presence(&self) {
        unsafe {
            sys::SteamAPI_ISteamFriends_ClearRichPresence(self.friends);
        }
    }
}

/// Information about a friend's current state in a game
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FriendGame {
    /// The id of the game that the friend is
    /// playing
    pub game: GameId,
    /// The address of the server the player is in
    pub game_address: Ipv4Addr,
    /// The game port of the server the player is in
    pub game_port: u16,
    /// The query port of the server the player is in
    pub query_port: u16,
    /// Optional id of the lobby the player is in
    pub lobby: LobbyId,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PersonaStateChange {
    pub steam_id: SteamId,
    pub flags: PersonaChange,
}

impl_callback!(cb: PersonaStateChange_t => PersonaStateChange {
    Self {
        steam_id: SteamId(cb.m_ulSteamID),
        flags: PersonaChange::from_bits_truncate(cb.m_nChangeFlags as i32),
    }
});

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GameOverlayActivated {
    pub active: bool,
}

impl_callback!(cb: GameOverlayActivated_t => GameOverlayActivated {
    Self {
        active: cb.m_bActive == 1,
    }
});

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GameLobbyJoinRequested {
    pub lobby_steam_id: LobbyId,
    pub friend_steam_id: SteamId,
}

impl_callback!(cb: GameLobbyJoinRequested_t => GameLobbyJoinRequested {
    Self {
        lobby_steam_id: LobbyId(cb.m_steamIDLobby.m_steamid.m_unAll64Bits),
        friend_steam_id: SteamId(cb.m_steamIDFriend.m_steamid.m_unAll64Bits),
    }
});

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GameRichPresenceJoinRequested {
    /// If you are joining a friend/being invited from a friend, this `SteamId` will be of said friend.
    /// If it's not coming from a friend, this `SteamId` will be invalid, which you can check with the `.is_invalid()`
    /// method.
    pub friend_steam_id: SteamId,
    /// the connect string, holding custom data to join a game or friend
    pub connect: String,
}

impl_callback!(cb: GameRichPresenceJoinRequested_t => GameRichPresenceJoinRequested {
    // Convert from &[i8] to &[u8] because c_char in C is signed.
    // Technically, this C string does not have to be UTF-8, but I think for all realistic uses it will be.
    let as_bytes = cb.m_rgchConnect.map(|c| c as u8);
    let connect = CStr::from_bytes_until_nul(&as_bytes)
        .expect("Connect string payload was not a valid C string");
    let connect = connect
        .to_str()
        .expect("Connect string payload was not valid UTF-8")
        .to_string();

    let friend_steam_id = SteamId::from_raw(cb.m_steamIDFriend.m_steamid.m_unAll64Bits);

    GameRichPresenceJoinRequested {
        friend_steam_id,
        connect,
    }
});

pub struct Friend {
    id: SteamId,
    friends: *mut sys::ISteamFriends,
    _inner: Arc<Inner>,
}

impl Debug for Friend {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Friend({:?})", self.id)
    }
}

impl Friend {
    pub fn id(&self) -> SteamId {
        self.id
    }

    pub fn name(&self) -> String {
        unsafe {
            let name = sys::SteamAPI_ISteamFriends_GetFriendPersonaName(self.friends, self.id.0);
            let name = CStr::from_ptr(name);
            name.to_string_lossy().into_owned()
        }
    }
    /// Gets the nickname that the current user has set for the specified user.
    pub fn nick_name(&self) -> Option<String> {
        unsafe {
            let name = sys::SteamAPI_ISteamFriends_GetPlayerNickname(self.friends, self.id.0);
            if name.is_null() {
                return None;
            }

            let name = CStr::from_ptr(name);
            if name.is_empty() {
                None
            } else {
                Some(name.to_string_lossy().into_owned())
            }
        }
    }

    pub fn state(&self) -> FriendState {
        unsafe {
            let state = sys::SteamAPI_ISteamFriends_GetFriendPersonaState(self.friends, self.id.0);
            friend_state_from_sys(state)
        }
    }

    /// Returns the relationship between the local user and this user (friend, blocked, pending request, ...).
    ///
    /// Returns `None` for the deprecated `Suggested` value and for any value this binding does not name.
    pub fn relationship(&self) -> Option<FriendRelationship> {
        let relationship =
            unsafe { sys::SteamAPI_ISteamFriends_GetFriendRelationship(self.friends, self.id.0) };
        relationship_from_sys(relationship)
    }

    /// Returns an earlier persona name of this user. `index` 0 is the most recent name before the current one.
    ///
    /// Returns `None` when there is no name at that index, which is how the SDK reports the end of the history.
    pub fn persona_name_history(&self, index: i32) -> Option<String> {
        unsafe {
            read_c_str(sys::SteamAPI_ISteamFriends_GetFriendPersonaNameHistory(
                self.friends,
                self.id.0,
                index,
            ))
        }
    }

    /// Returns the Steam level of this user. The local Steam client reports it only for users it knows well, so the
    /// value can be `0` until Steam has the data (unverified against a live Steam client).
    pub fn steam_level(&self) -> i32 {
        unsafe { sys::SteamAPI_ISteamFriends_GetFriendSteamLevel(self.friends, self.id.0) }
    }

    /// Returns the Rich Presence value this user has set for `key` (for example `"status"`).
    ///
    /// Returns `Ok(None)` when the user has not set the key. Returns `Err(())` when `key` contains a NUL byte.
    pub fn rich_presence(&self, key: &str) -> Result<Option<String>, ()> {
        let key = CString::new(key).map_err(|_| ())?;
        unsafe {
            Ok(read_c_str(
                sys::SteamAPI_ISteamFriends_GetFriendRichPresence(
                    self.friends,
                    self.id.0,
                    key.as_ptr(),
                ),
            ))
        }
    }

    /// Returns how many Rich Presence keys this user currently has. Used with [`Friend::rich_presence_key`] to list them.
    pub fn rich_presence_key_count(&self) -> i32 {
        unsafe {
            sys::SteamAPI_ISteamFriends_GetFriendRichPresenceKeyCount(self.friends, self.id.0)
        }
    }

    /// Returns the Rich Presence key at `index` (`0` to [`Friend::rich_presence_key_count`] minus one), or `None` when there is none.
    pub fn rich_presence_key(&self, index: i32) -> Option<String> {
        unsafe {
            read_c_str(sys::SteamAPI_ISteamFriends_GetFriendRichPresenceKeyByIndex(
                self.friends,
                self.id.0,
                index,
            ))
        }
    }

    /// Asks Steam for this user's Rich Presence. Steam answers later, and the `FriendRichPresenceUpdate` callback is
    /// posted when the data arrives (through `process_callbacks` or the JS `runCallbacks()`).
    pub fn request_rich_presence(&self) {
        unsafe {
            sys::SteamAPI_ISteamFriends_RequestFriendRichPresence(self.friends, self.id.0);
        }
    }

    /// Returns information about the game the player is current playing if any
    pub fn game_played(&self) -> Option<FriendGame> {
        unsafe {
            let mut info: sys::FriendGameInfo_t = std::mem::zeroed();
            if sys::SteamAPI_ISteamFriends_GetFriendGamePlayed(self.friends, self.id.0, &mut info) {
                Some(FriendGame {
                    game: GameId(std::mem::transmute(info.m_gameID)),
                    game_address: info.m_unGameIP.into(),
                    game_port: info.m_usGamePort,
                    query_port: info.m_usQueryPort,
                    lobby: LobbyId(info.m_steamIDLobby.m_steamid.m_unAll64Bits),
                })
            } else {
                None
            }
        }
    }
    /// Gets the app ID of the game that user played with someone on their recently-played-with list.
    pub fn coplay_game_played(&self) -> AppId {
        unsafe {
            let app_id = sys::SteamAPI_ISteamFriends_GetFriendCoplayGame(self.friends, self.id.0);
            AppId(app_id)
        }
    }

    /// Gets the timestamp of when the user played with someone on their recently-played-with list.
    pub fn coplay_time(&self) -> i32 {
        unsafe { sys::SteamAPI_ISteamFriends_GetFriendCoplayTime(self.friends, self.id.0) }
    }

    /// Returns a small (32x32) avatar for the user in RGBA format
    pub fn small_avatar(&self) -> Option<Vec<u8>> {
        unsafe {
            let utils = sys::SteamAPI_SteamUtils_v010();
            let img = sys::SteamAPI_ISteamFriends_GetSmallFriendAvatar(self.friends, self.id.0);
            if img == 0 {
                return None;
            }
            let mut width = 0;
            let mut height = 0;
            if !sys::SteamAPI_ISteamUtils_GetImageSize(utils, img, &mut width, &mut height) {
                return None;
            }
            let mut dest = vec![0; 32 * 32 * 4];
            if !sys::SteamAPI_ISteamUtils_GetImageRGBA(utils, img, dest.as_mut_ptr(), 32 * 32 * 4) {
                return None;
            }
            Some(dest)
        }
    }

    /// Returns a medium (64x64) avatar for the user in RGBA format
    pub fn medium_avatar(&self) -> Option<Vec<u8>> {
        unsafe {
            let utils = sys::SteamAPI_SteamUtils_v010();
            let img = sys::SteamAPI_ISteamFriends_GetMediumFriendAvatar(self.friends, self.id.0);
            if img == 0 {
                return None;
            }
            let mut width = 0;
            let mut height = 0;
            if !sys::SteamAPI_ISteamUtils_GetImageSize(utils, img, &mut width, &mut height) {
                return None;
            }
            let mut dest = vec![0; 64 * 64 * 4];
            if !sys::SteamAPI_ISteamUtils_GetImageRGBA(utils, img, dest.as_mut_ptr(), 64 * 64 * 4) {
                return None;
            }
            Some(dest)
        }
    }

    /// Returns a large (184x184) avatar for the user in RGBA format
    pub fn large_avatar(&self) -> Option<Vec<u8>> {
        unsafe {
            let utils = sys::SteamAPI_SteamUtils_v010();
            let img = sys::SteamAPI_ISteamFriends_GetLargeFriendAvatar(self.friends, self.id.0);
            if img == 0 {
                return None;
            }
            let mut width = 0;
            let mut height = 0;
            if !sys::SteamAPI_ISteamUtils_GetImageSize(utils, img, &mut width, &mut height) {
                return None;
            }
            let mut dest = vec![0; 184 * 184 * 4];
            if !sys::SteamAPI_ISteamUtils_GetImageRGBA(utils, img, dest.as_mut_ptr(), 184 * 184 * 4)
            {
                return None;
            }
            Some(dest)
        }
    }

    /// Checks if the user meets the specified criteria. (Friends, blocked, users on the same server, etc)
    pub fn has_friend(&self, flags: FriendFlags) -> bool {
        unsafe { sys::SteamAPI_ISteamFriends_HasFriend(self.friends, self.id.0, flags.bits() as _) }
    }

    /// Invites a friend or clan member to the current game using a special invite string.
    /// If the target user accepts the invite then the ConnectString gets added to the command-line when launching the game.
    /// If the game is already running for that user, then they will receive a GameRichPresenceJoinRequested_t callback with the connect string.
    pub fn invite_user_to_game(&self, connect_string: &str) {
        unsafe {
            let connect_string = CString::new(connect_string).unwrap();
            sys::SteamAPI_ISteamFriends_InviteUserToGame(
                self.friends,
                self.id.0,
                connect_string.as_ptr(),
            );
        }
    }

    /// Mark a target user as 'played with'.
    /// NOTE: The current user must be in game with the other player for the association to work.
    pub fn set_played_with(&self) {
        unsafe {
            sys::SteamAPI_ISteamFriends_SetPlayedWith(self.friends, self.id.0);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FriendState {
    Offline,
    Online,
    Invisible,
    Busy,
    Away,
    Snooze,
    LookingToTrade,
    LookingToPlay,
}

/// Maximum size of one chat message read with [`Friends::clan_chat_message`] or [`Friends::friend_message`].
/// The SDK header states no maximum, so a longer message is cut at this many bytes (unverified against a live Steam client).
const CHAT_MESSAGE_BUFFER_LEN: usize = 8192;

/// The relationship between the local user and another user (`EFriendRelationship`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FriendRelationship {
    None,
    Blocked,
    RequestRecipient,
    Friend,
    RequestInitiator,
    Ignored,
    IgnoredFriend,
}

/// Identifies one of the local user's friends groups (`FriendsGroupID_t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FriendsGroupId(pub(crate) i16);

impl FriendsGroupId {
    /// Creates a `FriendsGroupId` from its raw value, as returned by [`Friends::friends_group_id`].
    pub fn from_raw(id: i16) -> FriendsGroupId {
        FriendsGroupId(id)
    }

    /// Returns the raw value of the group id.
    pub fn raw(&self) -> i16 {
        self.0
    }
}

/// One of the local user's friends groups, with its members.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FriendsGroup {
    /// The group id.
    pub id: FriendsGroupId,
    /// The group name, or an empty string when Steam returns none.
    pub name: String,
    /// The members of the group, as far as the local Steam client knows them.
    pub members: Vec<SteamId>,
}

/// How recently a clan was active, as the local Steam client last knew it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ClanActivityCounts {
    /// Members online.
    pub online: i32,
    /// Members in a game.
    pub in_game: i32,
    /// Members in the clan chat.
    pub chatting: i32,
}

/// The Steam Community items a user has equipped on their profile (`EquippedProfileItems_t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EquippedProfileItems {
    pub has_animated_avatar: bool,
    pub has_avatar_frame: bool,
    pub has_profile_modifier: bool,
    pub has_profile_background: bool,
    pub has_mini_profile_background: bool,
    /// `true` when Steam answered from its local cache instead of asking the server.
    pub from_cache: bool,
}

/// One page of the users that follow someone or that the local user follows (`FriendsEnumerateFollowingList_t`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FollowingList {
    /// The SteamIDs on this page, at most 50.
    pub steam_ids: Vec<SteamId>,
    /// The total number of followers in the list, across all pages.
    pub total_count: i32,
}

/// A message in a clan chat or a friend chat, read with [`Friends::clan_chat_message`] or [`Friends::friend_message`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ChatMessage {
    /// The user who sent the message. For a friend message this is the friend.
    pub chatter: SteamId,
    /// The kind of entry (a chat message, an invite, a join or leave notice, ...).
    pub entry_type: ChatEntryType,
    /// The text of the message, cut at the first NUL byte. It is empty for entries that carry no text.
    pub text: String,
}

/// The kind of a Community profile item (`ECommunityProfileItemType`), used with [`Friends::has_equipped_profile_item`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ProfileItemType {
    AnimatedAvatar,
    AvatarFrame,
    ProfileModifier,
    ProfileBackground,
    MiniProfileBackground,
}

/// A property of a Community profile item (`ECommunityProfileItemProperty`).
///
/// Use the string properties with [`Friends::profile_item_property_string`] and the `AppId`, `TypeId` and `Class`
/// properties with [`Friends::profile_item_property_uint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ProfileItemProperty {
    ImageSmall,
    ImageLarge,
    InternalName,
    Title,
    Description,
    AppId,
    TypeId,
    Class,
    MovieWebM,
    MovieMp4,
    MovieWebMSmall,
    MovieMp4Small,
}

impl Friends {
    /// Returns the local user's own persona state (online, away, invisible, ...), as the local Steam client reports it.
    /// Requires a running Steam client.
    pub fn persona_state(&self) -> FriendState {
        unsafe { friend_state_from_sys(sys::SteamAPI_ISteamFriends_GetPersonaState(self.friends)) }
    }

    /// Returns the number of friends groups the local user has.
    pub fn friends_group_count(&self) -> i32 {
        unsafe { sys::SteamAPI_ISteamFriends_GetFriendsGroupCount(self.friends) }
    }

    /// Returns the id of the friends group at `index` (`0` to [`Friends::friends_group_count`] minus one), or `None` when
    /// the index is not a group.
    pub fn friends_group_id(&self, index: i32) -> Option<FriendsGroupId> {
        let id =
            unsafe { sys::SteamAPI_ISteamFriends_GetFriendsGroupIDByIndex(self.friends, index) };
        // -1 is k_FriendsGroupID_Invalid in the SDK header
        (id != -1).then_some(FriendsGroupId(id))
    }

    /// Returns the name of a friends group, or `None` when the id is not a group or the name is empty.
    pub fn friends_group_name(&self, group: FriendsGroupId) -> Option<String> {
        unsafe {
            read_c_str(sys::SteamAPI_ISteamFriends_GetFriendsGroupName(
                self.friends,
                group.0,
            ))
        }
    }

    /// Returns the number of members in a friends group.
    pub fn friends_group_members_count(&self, group: FriendsGroupId) -> i32 {
        unsafe { sys::SteamAPI_ISteamFriends_GetFriendsGroupMembersCount(self.friends, group.0) }
    }

    /// Returns the members of a friends group. Slots the SDK leaves empty are skipped, so the list can be shorter
    /// than [`Friends::friends_group_members_count`].
    pub fn friends_group_members(&self, group: FriendsGroupId) -> Vec<SteamId> {
        let count = self.friends_group_members_count(group);
        let Ok(count) = usize::try_from(count) else {
            return Vec::new();
        };
        let mut members = vec![steam_id_to_sys(SteamId(0)); count];
        unsafe {
            sys::SteamAPI_ISteamFriends_GetFriendsGroupMembersList(
                self.friends,
                group.0,
                members.as_mut_ptr(),
                count as i32,
            );
        }
        members
            .into_iter()
            .map(steam_id_from_sys)
            .filter(|id| !id.is_invalid())
            .collect()
    }

    /// Returns every friends group with its members, in index order.
    pub fn friends_groups(&self) -> Vec<FriendsGroup> {
        (0..self.friends_group_count())
            .filter_map(|index| {
                let id = self.friends_group_id(index)?;
                Some(FriendsGroup {
                    id,
                    name: self.friends_group_name(id).unwrap_or_default(),
                    members: self.friends_group_members(id),
                })
            })
            .collect()
    }

    /// Returns the number of clans (groups) the local user is a member of.
    pub fn clan_count(&self) -> i32 {
        unsafe { sys::SteamAPI_ISteamFriends_GetClanCount(self.friends) }
    }

    /// Returns the SteamID of the clan at `index` (`0` to [`Friends::clan_count`] minus one), or `None` when the SDK returns an invalid id.
    pub fn clan_by_index(&self, index: i32) -> Option<SteamId> {
        let id =
            SteamId(unsafe { sys::SteamAPI_ISteamFriends_GetClanByIndex(self.friends, index) });
        (!id.is_invalid()).then_some(id)
    }

    /// Returns the name of a clan, or `None` when Steam does not know it yet or the name is empty.
    /// For a clan the user is not a member of, call [`Friends::download_clan_activity_counts`] first (the name is not part of that call; unverified).
    pub fn clan_name(&self, clan: SteamId) -> Option<String> {
        unsafe {
            read_c_str(sys::SteamAPI_ISteamFriends_GetClanName(
                self.friends,
                clan.0,
            ))
        }
    }

    /// Returns the tag of a clan, or `None` when Steam does not know it yet or the tag is empty.
    pub fn clan_tag(&self, clan: SteamId) -> Option<String> {
        unsafe { read_c_str(sys::SteamAPI_ISteamFriends_GetClanTag(self.friends, clan.0)) }
    }

    /// Returns the latest activity counts Steam knows for a clan, or `None` when it has none.
    /// Clans the user is not in only have counts after [`Friends::download_clan_activity_counts`] has completed.
    pub fn clan_activity_counts(&self, clan: SteamId) -> Option<ClanActivityCounts> {
        let (mut online, mut in_game, mut chatting) = (0i32, 0i32, 0i32);
        let known = unsafe {
            sys::SteamAPI_ISteamFriends_GetClanActivityCounts(
                self.friends,
                clan.0,
                &mut online,
                &mut in_game,
                &mut chatting,
            )
        };
        known.then_some(ClanActivityCounts {
            online,
            in_game,
            chatting,
        })
    }

    /// Asks Steam for the activity counts of clans, including clans the user is not a member of.
    ///
    /// `cb` gets `Ok(())` when Steam reports success, `Err(SteamError::Generic)` when it reports failure, and
    /// `Err(SteamError::InvalidParameter)` at once when there are more clans than an `i32` count can hold.
    pub fn download_clan_activity_counts<F>(&self, clans: &[SteamId], cb: F)
    where
        F: FnOnce(Result<(), SteamError>) + 'static + Send,
    {
        let Ok(count) = i32::try_from(clans.len()) else {
            cb(Err(SteamError::InvalidParameter));
            return;
        };
        let mut ids: Vec<sys::CSteamID> = clans.iter().map(|id| steam_id_to_sys(*id)).collect();
        unsafe {
            let api_call = sys::SteamAPI_ISteamFriends_DownloadClanActivityCounts(
                self.friends,
                ids.as_mut_ptr(),
                count,
            );
            register_call_result::<sys::DownloadClanActivityCountsResult_t, _>(
                &self.inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else if v.m_bSuccess {
                        Ok(())
                    } else {
                        Err(SteamError::Generic)
                    })
                },
            );
        }
    }

    /// Asks Steam for a clan's officer list, which gives the owner and the officers (see [`Friends::clan_owner`],
    /// [`Friends::clan_officer_count`] and [`Friends::clan_officer_by_index`]). The local user must be a member of the clan.
    ///
    /// `cb` gets the number of officers, the owner included, when Steam reports success, and `Err(SteamError::Generic)`
    /// when it reports failure.
    pub fn request_clan_officer_list<F>(&self, clan: SteamId, cb: F)
    where
        F: FnOnce(Result<i32, SteamError>) + 'static + Send,
    {
        unsafe {
            let api_call = sys::SteamAPI_ISteamFriends_RequestClanOfficerList(self.friends, clan.0);
            register_call_result::<sys::ClanOfficerListResponse_t, _>(
                &self.inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else if v.m_bSuccess != 0 {
                        Ok(v.m_cOfficers)
                    } else {
                        Err(SteamError::Generic)
                    })
                },
            );
        }
    }

    /// Returns the owner of a clan. Valid only after [`Friends::request_clan_officer_list`] has completed.
    /// Returns `None` when the owner is not known yet.
    pub fn clan_owner(&self, clan: SteamId) -> Option<SteamId> {
        let id = SteamId(unsafe { sys::SteamAPI_ISteamFriends_GetClanOwner(self.friends, clan.0) });
        (!id.is_invalid()).then_some(id)
    }

    /// Returns the number of officers of a clan, the owner included. Valid only after [`Friends::request_clan_officer_list`] has completed.
    pub fn clan_officer_count(&self, clan: SteamId) -> i32 {
        unsafe { sys::SteamAPI_ISteamFriends_GetClanOfficerCount(self.friends, clan.0) }
    }

    /// Returns the officer at `index` (`0` to [`Friends::clan_officer_count`] minus one). Valid only after
    /// [`Friends::request_clan_officer_list`] has completed. Returns `None` for an invalid id.
    pub fn clan_officer_by_index(&self, clan: SteamId, index: i32) -> Option<SteamId> {
        let id = SteamId(unsafe {
            sys::SteamAPI_ISteamFriends_GetClanOfficerByIndex(self.friends, clan.0, index)
        });
        (!id.is_invalid()).then_some(id)
    }

    /// Returns whether a clan is public. Unknown clans are reported as not public (unverified against a live Steam client).
    pub fn is_clan_public(&self, clan: SteamId) -> bool {
        unsafe { sys::SteamAPI_ISteamFriends_IsClanPublic(self.friends, clan.0) }
    }

    /// Returns whether a clan is the official game group of an app (unverified against a live Steam client).
    pub fn is_clan_official_game_group(&self, clan: SteamId) -> bool {
        unsafe { sys::SteamAPI_ISteamFriends_IsClanOfficialGameGroup(self.friends, clan.0) }
    }

    /// Returns how many users the local user can see in a source: a clan, lobby, chat room or game server.
    /// The current user must be in the source for the list to be complete (unverified against a live Steam client).
    pub fn friend_count_from_source(&self, source: SteamId) -> i32 {
        unsafe { sys::SteamAPI_ISteamFriends_GetFriendCountFromSource(self.friends, source.0) }
    }

    /// Returns the user at `index` in a source (see [`Friends::friend_count_from_source`]), or `None` for an invalid id.
    pub fn friend_from_source_by_index(&self, source: SteamId, index: i32) -> Option<SteamId> {
        let id = SteamId(unsafe {
            sys::SteamAPI_ISteamFriends_GetFriendFromSourceByIndex(self.friends, source.0, index)
        });
        (!id.is_invalid()).then_some(id)
    }

    /// Returns whether the local user can see that `user` is a member of, or in, `source`.
    pub fn is_user_in_source(&self, user: SteamId, source: SteamId) -> bool {
        unsafe { sys::SteamAPI_ISteamFriends_IsUserInSource(self.friends, user.0, source.0) }
    }

    /// Tells Steam that `user` is pressing the talk button in the current game, which mutes the microphone of the Steam
    /// friends voice chat for that user. Only the local Steam client's friends UI is affected (unverified against a live Steam client).
    pub fn set_in_game_voice_speaking(&self, user: SteamId, speaking: bool) {
        unsafe {
            sys::SteamAPI_ISteamFriends_SetInGameVoiceSpeaking(self.friends, user.0, speaking);
        }
    }

    /// Returns the number of chats (friends or chat rooms) with unread messages that would show a notification.
    /// Watch the `UnreadChatMessagesChanged` callback to know when this changes.
    pub fn num_chats_with_unread_priority_messages(&self) -> i32 {
        unsafe { sys::SteamAPI_ISteamFriends_GetNumChatsWithUnreadPriorityMessages(self.friends) }
    }

    /// Opens the overlay's Remote Play Together invite dialog for a lobby. The user sends the invitations in the dialog.
    /// Requires a running Steam client and the overlay.
    pub fn activate_remote_play_together_invite_dialog(&self, lobby: LobbyId) {
        unsafe {
            sys::SteamAPI_ISteamFriends_ActivateGameOverlayRemotePlayTogetherInviteDialog(
                self.friends,
                lobby.0,
            );
        }
    }

    /// Makes the overlay browser report navigations to `protocol` (for example `"myapp"`) instead of opening them. Steam
    /// then posts the `OverlayBrowserProtocolNavigation` callback. Call it before opening a page in modal mode.
    ///
    /// Returns `Ok(true)` when Steam accepted the protocol and `Err(())` when `protocol` contains a NUL byte.
    pub fn register_protocol_in_overlay_browser(&self, protocol: &str) -> Result<bool, ()> {
        let protocol = CString::new(protocol).map_err(|_| ())?;
        unsafe {
            Ok(
                sys::SteamAPI_ISteamFriends_RegisterProtocolInOverlayBrowser(
                    self.friends,
                    protocol.as_ptr(),
                ),
            )
        }
    }

    /// Joins a clan chat room from inside the game. `cb` gets the room's response: check for
    /// [`ChatRoomEnterResponse::Success`]. `Err(SteamError::IOFailure)` means no answer arrived.
    pub fn join_clan_chat_room<F>(&self, clan: SteamId, cb: F)
    where
        F: FnOnce(Result<ChatRoomEnterResponse, SteamError>) + 'static + Send,
    {
        unsafe {
            let api_call = sys::SteamAPI_ISteamFriends_JoinClanChatRoom(self.friends, clan.0);
            register_call_result_raw::<sys::JoinClanChatRoomCompletionResult_t, _>(
                &self.inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else {
                        // The response is read as an integer, so a value the binding does not name cannot make an invalid enum
                        let raw = std::ptr::addr_of!((*v).m_eChatRoomEnterResponse)
                            .cast::<u32>()
                            .read_unaligned();
                        Ok(ChatRoomEnterResponse::from(raw))
                    })
                },
            );
        }
    }

    /// Leaves a clan chat room the game has joined. Returns `false` when Steam refuses (for example the room is not joined).
    pub fn leave_clan_chat_room(&self, clan: SteamId) -> bool {
        unsafe { sys::SteamAPI_ISteamFriends_LeaveClanChatRoom(self.friends, clan.0) }
    }

    /// Returns the number of members in a clan chat room the game has joined.
    pub fn clan_chat_member_count(&self, clan_chat: SteamId) -> i32 {
        unsafe { sys::SteamAPI_ISteamFriends_GetClanChatMemberCount(self.friends, clan_chat.0) }
    }

    /// Returns the member at `index` of a clan chat room (`0` to [`Friends::clan_chat_member_count`] minus one), or `None` for an invalid id.
    pub fn clan_chat_member_by_index(&self, clan_chat: SteamId, index: i32) -> Option<SteamId> {
        let id = SteamId(unsafe {
            sys::SteamAPI_ISteamFriends_GetChatMemberByIndex(self.friends, clan_chat.0, index)
        });
        (!id.is_invalid()).then_some(id)
    }

    /// Sends `text` to a clan chat room the game has joined. Returns `Ok(true)` when Steam accepted it, and `Err(())`
    /// when `text` contains a NUL byte. The message is posted to the clan chat for every member: this is a send.
    pub fn send_clan_chat_message(&self, clan_chat: SteamId, text: &str) -> Result<bool, ()> {
        let text = CString::new(text).map_err(|_| ())?;
        unsafe {
            Ok(sys::SteamAPI_ISteamFriends_SendClanChatMessage(
                self.friends,
                clan_chat.0,
                text.as_ptr(),
            ))
        }
    }

    /// Reads message `index` of a clan chat room, with the sender and the entry type.
    ///
    /// Returns `None` when Steam returns a negative length. Whether an out-of-range index gives `None` or an empty entry is
    /// unverified against a live Steam client. Messages longer than 8192 bytes are cut.
    pub fn clan_chat_message(&self, clan_chat: SteamId, index: i32) -> Option<ChatMessage> {
        let mut buffer = vec![0u8; CHAT_MESSAGE_BUFFER_LEN];
        let mut entry_type = 0u32;
        let mut chatter = steam_id_to_sys(SteamId(0));
        let len = unsafe {
            sys::SteamAPI_ISteamFriends_GetClanChatMessage(
                self.friends,
                clan_chat.0,
                index,
                buffer.as_mut_ptr().cast(),
                CHAT_MESSAGE_BUFFER_LEN as i32,
                (&mut entry_type as *mut u32).cast(),
                &mut chatter,
            )
        };
        let text = chat_text(&buffer, len)?;
        Some(ChatMessage {
            chatter: steam_id_from_sys(chatter),
            entry_type: chat_entry_type_from_raw(entry_type),
            text,
        })
    }

    /// Returns whether `user` is an admin of a clan chat room.
    pub fn is_clan_chat_admin(&self, clan_chat: SteamId, user: SteamId) -> bool {
        unsafe { sys::SteamAPI_ISteamFriends_IsClanChatAdmin(self.friends, clan_chat.0, user.0) }
    }

    /// Returns whether the clan chat window is open in the Steam client (outside the game).
    pub fn is_clan_chat_window_open_in_steam(&self, clan_chat: SteamId) -> bool {
        unsafe {
            sys::SteamAPI_ISteamFriends_IsClanChatWindowOpenInSteam(self.friends, clan_chat.0)
        }
    }

    /// Opens the clan chat window in the Steam client. Returns `false` when Steam refuses. This changes what the user sees.
    pub fn open_clan_chat_window_in_steam(&self, clan_chat: SteamId) -> bool {
        unsafe { sys::SteamAPI_ISteamFriends_OpenClanChatWindowInSteam(self.friends, clan_chat.0) }
    }

    /// Closes the clan chat window in the Steam client. Returns `false` when Steam refuses. This changes what the user sees.
    pub fn close_clan_chat_window_in_steam(&self, clan_chat: SteamId) -> bool {
        unsafe { sys::SteamAPI_ISteamFriends_CloseClanChatWindowInSteam(self.friends, clan_chat.0) }
    }

    /// Turns on or off the interception of friend-to-friend chat messages. While it is on, the game gets the messages
    /// through the `GameConnectedFriendChatMsg` callback and reads them with [`Friends::friend_message`], and the Steam
    /// client does not show them. Returns `false` when Steam refuses.
    pub fn set_listen_for_friends_messages(&self, enabled: bool) -> bool {
        unsafe { sys::SteamAPI_ISteamFriends_SetListenForFriendsMessages(self.friends, enabled) }
    }

    /// Sends a chat message to a friend. Returns `Ok(true)` when Steam accepted it, and `Err(())` when `text` contains a NUL byte.
    /// The message is delivered to the friend: this is a send.
    pub fn reply_to_friend_message(&self, friend: SteamId, text: &str) -> Result<bool, ()> {
        let text = CString::new(text).map_err(|_| ())?;
        unsafe {
            Ok(sys::SteamAPI_ISteamFriends_ReplyToFriendMessage(
                self.friends,
                friend.0,
                text.as_ptr(),
            ))
        }
    }

    /// Reads message `index` of a friend chat, with the entry type. The sender is `friend`.
    ///
    /// Returns `None` when Steam returns a negative length. Messages longer than 8192 bytes are cut. Use it after the
    /// `GameConnectedFriendChatMsg` callback, with that callback's message id.
    pub fn friend_message(&self, friend: SteamId, index: i32) -> Option<ChatMessage> {
        let mut buffer = vec![0u8; CHAT_MESSAGE_BUFFER_LEN];
        let mut entry_type = 0u32;
        let len = unsafe {
            sys::SteamAPI_ISteamFriends_GetFriendMessage(
                self.friends,
                friend.0,
                index,
                buffer.as_mut_ptr().cast(),
                CHAT_MESSAGE_BUFFER_LEN as i32,
                (&mut entry_type as *mut u32).cast(),
            )
        };
        let text = chat_text(&buffer, len)?;
        Some(ChatMessage {
            chatter: friend,
            entry_type: chat_entry_type_from_raw(entry_type),
            text,
        })
    }

    /// Asks Steam how many users follow `steam_id`. `cb` gets the count, or the Steam error.
    pub fn follower_count<F>(&self, steam_id: SteamId, cb: F)
    where
        F: FnOnce(Result<i32, SteamError>) + 'static + Send,
    {
        unsafe {
            let api_call = sys::SteamAPI_ISteamFriends_GetFollowerCount(self.friends, steam_id.0);
            register_call_result_raw::<sys::FriendsGetFollowerCount_t, _>(
                &self.inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else {
                        // SAFETY: io_error is false, so v points at a full FriendsGetFollowerCount_t
                        let raw = std::ptr::addr_of!((*v).m_eResult)
                            .cast::<u32>()
                            .read_unaligned();
                        crate::error::check_raw_result(raw).map(|_| (*v).m_nCount)
                    })
                },
            );
        }
    }

    /// Asks Steam whether the local user follows `steam_id`. `cb` gets the answer, or the Steam error.
    pub fn is_following<F>(&self, steam_id: SteamId, cb: F)
    where
        F: FnOnce(Result<bool, SteamError>) + 'static + Send,
    {
        unsafe {
            let api_call = sys::SteamAPI_ISteamFriends_IsFollowing(self.friends, steam_id.0);
            register_call_result_raw::<sys::FriendsIsFollowing_t, _>(
                &self.inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else {
                        // SAFETY: io_error is false, so v points at a full FriendsIsFollowing_t
                        let raw = std::ptr::addr_of!((*v).m_eResult)
                            .cast::<u32>()
                            .read_unaligned();
                        crate::error::check_raw_result(raw).map(|_| (*v).m_bIsFollowing)
                    })
                },
            );
        }
    }

    /// Asks Steam for one page of the users the local user follows, starting at `start_index`. Steam returns at most 50 per page.
    pub fn enumerate_following_list<F>(&self, start_index: u32, cb: F)
    where
        F: FnOnce(Result<FollowingList, SteamError>) + 'static + Send,
    {
        unsafe {
            let api_call =
                sys::SteamAPI_ISteamFriends_EnumerateFollowingList(self.friends, start_index);
            register_call_result_raw::<sys::FriendsEnumerateFollowingList_t, _>(
                &self.inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else {
                        // SAFETY: io_error is false, so v points at a full FriendsEnumerateFollowingList_t
                        let raw = std::ptr::addr_of!((*v).m_eResult)
                            .cast::<u32>()
                            .read_unaligned();
                        crate::error::check_raw_result(raw).map(|_| {
                            let page = v.read_unaligned();
                            let ids = page.m_rgSteamID;
                            let returned = usize::try_from(page.m_nResultsReturned)
                                .unwrap_or(0)
                                .min(ids.len());
                            FollowingList {
                                steam_ids: ids[..returned]
                                    .iter()
                                    .map(|id| steam_id_from_sys(*id))
                                    .collect(),
                                total_count: page.m_nTotalResultCount,
                            }
                        })
                    })
                },
            );
        }
    }

    /// Asks Steam for the Community profile items a user has equipped. Steam posts the `EquippedProfileItemsChanged`
    /// callback when a friend changes them. `cb` gets the flags, or the Steam error.
    pub fn request_equipped_profile_items<F>(&self, steam_id: SteamId, cb: F)
    where
        F: FnOnce(Result<EquippedProfileItems, SteamError>) + 'static + Send,
    {
        unsafe {
            let api_call =
                sys::SteamAPI_ISteamFriends_RequestEquippedProfileItems(self.friends, steam_id.0);
            register_call_result_raw::<sys::EquippedProfileItems_t, _>(
                &self.inner,
                api_call,
                move |v, io_error| {
                    cb(if io_error {
                        Err(SteamError::IOFailure)
                    } else {
                        // SAFETY: io_error is false, so v points at a full EquippedProfileItems_t
                        let raw = std::ptr::addr_of!((*v).m_eResult)
                            .cast::<u32>()
                            .read_unaligned();
                        crate::error::check_raw_result(raw).map(|_| {
                            let items = v.read_unaligned();
                            EquippedProfileItems {
                                has_animated_avatar: items.m_bHasAnimatedAvatar,
                                has_avatar_frame: items.m_bHasAvatarFrame,
                                has_profile_modifier: items.m_bHasProfileModifier,
                                has_profile_background: items.m_bHasProfileBackground,
                                has_mini_profile_background: items.m_bHasMiniProfileBackground,
                                from_cache: items.m_bFromCache,
                            }
                        })
                    })
                },
            );
        }
    }

    /// Returns whether a user has equipped a profile item of that kind. Valid only after
    /// [`Friends::request_equipped_profile_items`] has completed for that user.
    pub fn has_equipped_profile_item(&self, steam_id: SteamId, item_type: ProfileItemType) -> bool {
        unsafe {
            sys::SteamAPI_ISteamFriends_BHasEquippedProfileItem(
                self.friends,
                steam_id.0,
                profile_item_type_to_sys(item_type),
            )
        }
    }

    /// Returns a string property of an equipped profile item (for example `Title`), or `None` when Steam has no value.
    /// Valid only after [`Friends::request_equipped_profile_items`] has completed for that user.
    pub fn profile_item_property_string(
        &self,
        steam_id: SteamId,
        item_type: ProfileItemType,
        property: ProfileItemProperty,
    ) -> Option<String> {
        unsafe {
            read_c_str(sys::SteamAPI_ISteamFriends_GetProfileItemPropertyString(
                self.friends,
                steam_id.0,
                profile_item_type_to_sys(item_type),
                profile_item_property_to_sys(property),
            ))
        }
    }

    /// Returns a numeric property of an equipped profile item (`AppId`, `TypeId` or `Class`). Returns `0` when Steam has no value.
    /// Valid only after [`Friends::request_equipped_profile_items`] has completed for that user.
    pub fn profile_item_property_uint(
        &self,
        steam_id: SteamId,
        item_type: ProfileItemType,
        property: ProfileItemProperty,
    ) -> u32 {
        unsafe {
            sys::SteamAPI_ISteamFriends_GetProfileItemPropertyUint(
                self.friends,
                steam_id.0,
                profile_item_type_to_sys(item_type),
                profile_item_property_to_sys(property),
            )
        }
    }
}

// Callback payloads for the friends interface. Call results (the `*_t` structs that answer a request) are decoded inside the
// methods above and are not public types.

/// A friend sent a chat message while [`Friends::set_listen_for_friends_messages`] is on. Read it with [`Friends::friend_message`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GameConnectedFriendChatMsg {
    /// The friend who sent the message.
    pub steam_id: SteamId,
    /// The index to pass to [`Friends::friend_message`].
    pub message_id: i32,
}

impl_callback!(cb: GameConnectedFriendChatMsg_t => GameConnectedFriendChatMsg {
    Self {
        steam_id: SteamId(cb.m_steamIDUser.m_steamid.m_unAll64Bits),
        message_id: cb.m_iMessageID,
    }
});

/// A chat message arrived in a clan chat room the game has joined. Read it with [`Friends::clan_chat_message`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GameConnectedClanChatMsg {
    /// The clan chat room.
    pub clan_chat: SteamId,
    /// The user who sent the message.
    pub user: SteamId,
    /// The index to pass to [`Friends::clan_chat_message`].
    pub message_id: i32,
}

impl_callback!(cb: GameConnectedClanChatMsg_t => GameConnectedClanChatMsg {
    Self {
        clan_chat: SteamId(cb.m_steamIDClanChat.m_steamid.m_unAll64Bits),
        user: SteamId(cb.m_steamIDUser.m_steamid.m_unAll64Bits),
        message_id: cb.m_iMessageID,
    }
});

/// A user joined a clan chat room the game has joined.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GameConnectedChatJoin {
    /// The clan chat room.
    pub clan_chat: SteamId,
    /// The user who joined.
    pub user: SteamId,
}

impl_callback!(cb: GameConnectedChatJoin_t => GameConnectedChatJoin {
    Self {
        clan_chat: SteamId(cb.m_steamIDClanChat.m_steamid.m_unAll64Bits),
        user: SteamId(cb.m_steamIDUser.m_steamid.m_unAll64Bits),
    }
});

/// A user left a clan chat room the game has joined.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GameConnectedChatLeave {
    /// The clan chat room.
    pub clan_chat: SteamId,
    /// The user who left.
    pub user: SteamId,
    /// `true` when an admin kicked the user.
    pub kicked: bool,
    /// `true` when the Steam connection dropped.
    pub dropped: bool,
}

impl_callback!(cb: GameConnectedChatLeave_t => GameConnectedChatLeave {
    Self {
        clan_chat: SteamId(cb.m_steamIDClanChat.m_steamid.m_unAll64Bits),
        user: SteamId(cb.m_steamIDUser.m_steamid.m_unAll64Bits),
        kicked: cb.m_bKicked,
        dropped: cb.m_bDropped,
    }
});

/// A friend's Rich Presence changed. Read it with [`Friend::rich_presence`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FriendRichPresenceUpdate {
    /// The friend whose Rich Presence changed.
    pub steam_id: SteamId,
    /// The app the friend is running. It is the current game when the friend plays it.
    pub app_id: AppId,
}

impl_callback!(cb: FriendRichPresenceUpdate_t => FriendRichPresenceUpdate {
    Self {
        steam_id: SteamId(cb.m_steamIDFriend.m_steamid.m_unAll64Bits),
        app_id: AppId(cb.m_nAppID),
    }
});

/// The number of chats with unread priority messages changed. Read it with [`Friends::num_chats_with_unread_priority_messages`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnreadChatMessagesChanged {}

impl_callback!(cb: UnreadChatMessagesChanged_t => UnreadChatMessagesChanged {
    let _ = cb;
    Self {}
});

/// The overlay browser navigated to a protocol registered with [`Friends::register_protocol_in_overlay_browser`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OverlayBrowserProtocolNavigation {
    /// The full address the browser tried to open, cut at the first NUL byte.
    pub uri: String,
}

impl_callback!(cb: OverlayBrowserProtocolNavigation_t => OverlayBrowserProtocolNavigation {
    // c_char is signed in C, so read the bytes as u8 first
    let bytes = cb.rgchURI.map(|c| c as u8);
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    Self {
        uri: String::from_utf8_lossy(&bytes[..end]).into_owned(),
    }
});

/// A friend changed their equipped profile items. Request them again with [`Friends::request_equipped_profile_items`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EquippedProfileItemsChanged {
    /// The friend whose items changed.
    pub steam_id: SteamId,
}

impl_callback!(cb: EquippedProfileItemsChanged_t => EquippedProfileItemsChanged {
    Self {
        steam_id: SteamId(cb.m_steamID.m_steamid.m_unAll64Bits),
    }
});

// Helpers for the friends interface

/// Maps the SDK persona state to [`FriendState`].
fn friend_state_from_sys(state: sys::EPersonaState) -> FriendState {
    match state {
        sys::EPersonaState::k_EPersonaStateOffline => FriendState::Offline,
        sys::EPersonaState::k_EPersonaStateOnline => FriendState::Online,
        sys::EPersonaState::k_EPersonaStateInvisible => FriendState::Invisible,
        sys::EPersonaState::k_EPersonaStateBusy => FriendState::Busy,
        sys::EPersonaState::k_EPersonaStateAway => FriendState::Away,
        sys::EPersonaState::k_EPersonaStateSnooze => FriendState::Snooze,
        sys::EPersonaState::k_EPersonaStateLookingToPlay => FriendState::LookingToPlay,
        sys::EPersonaState::k_EPersonaStateLookingToTrade => FriendState::LookingToTrade,
        _ => unreachable!(),
    }
}

/// Maps the SDK relationship to [`FriendRelationship`]. The deprecated `Suggested` value and unknown values give `None`.
fn relationship_from_sys(relationship: sys::EFriendRelationship) -> Option<FriendRelationship> {
    use sys::EFriendRelationship as R;
    Some(match relationship {
        R::k_EFriendRelationshipNone => FriendRelationship::None,
        R::k_EFriendRelationshipBlocked => FriendRelationship::Blocked,
        R::k_EFriendRelationshipRequestRecipient => FriendRelationship::RequestRecipient,
        R::k_EFriendRelationshipFriend => FriendRelationship::Friend,
        R::k_EFriendRelationshipRequestInitiator => FriendRelationship::RequestInitiator,
        R::k_EFriendRelationshipIgnored => FriendRelationship::Ignored,
        R::k_EFriendRelationshipIgnoredFriend => FriendRelationship::IgnoredFriend,
        _ => return None,
    })
}

fn profile_item_type_to_sys(item_type: ProfileItemType) -> sys::ECommunityProfileItemType {
    match item_type {
        ProfileItemType::AnimatedAvatar => {
            sys::ECommunityProfileItemType::k_ECommunityProfileItemType_AnimatedAvatar
        }
        ProfileItemType::AvatarFrame => {
            sys::ECommunityProfileItemType::k_ECommunityProfileItemType_AvatarFrame
        }
        ProfileItemType::ProfileModifier => {
            sys::ECommunityProfileItemType::k_ECommunityProfileItemType_ProfileModifier
        }
        ProfileItemType::ProfileBackground => {
            sys::ECommunityProfileItemType::k_ECommunityProfileItemType_ProfileBackground
        }
        ProfileItemType::MiniProfileBackground => {
            sys::ECommunityProfileItemType::k_ECommunityProfileItemType_MiniProfileBackground
        }
    }
}

fn profile_item_property_to_sys(
    property: ProfileItemProperty,
) -> sys::ECommunityProfileItemProperty {
    use sys::ECommunityProfileItemProperty as P;
    match property {
        ProfileItemProperty::ImageSmall => P::k_ECommunityProfileItemProperty_ImageSmall,
        ProfileItemProperty::ImageLarge => P::k_ECommunityProfileItemProperty_ImageLarge,
        ProfileItemProperty::InternalName => P::k_ECommunityProfileItemProperty_InternalName,
        ProfileItemProperty::Title => P::k_ECommunityProfileItemProperty_Title,
        ProfileItemProperty::Description => P::k_ECommunityProfileItemProperty_Description,
        ProfileItemProperty::AppId => P::k_ECommunityProfileItemProperty_AppID,
        ProfileItemProperty::TypeId => P::k_ECommunityProfileItemProperty_TypeID,
        ProfileItemProperty::Class => P::k_ECommunityProfileItemProperty_Class,
        ProfileItemProperty::MovieWebM => P::k_ECommunityProfileItemProperty_MovieWebM,
        ProfileItemProperty::MovieMp4 => P::k_ECommunityProfileItemProperty_MovieMP4,
        ProfileItemProperty::MovieWebMSmall => P::k_ECommunityProfileItemProperty_MovieWebMSmall,
        ProfileItemProperty::MovieMp4Small => P::k_ECommunityProfileItemProperty_MovieMP4Small,
    }
}

/// Copies a C string returned by the SDK. Returns `None` for a null pointer or an empty string.
fn read_c_str(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the SDK returns a NUL-terminated string; it is copied here before any other call into the interface
    let text = unsafe { CStr::from_ptr(ptr) };
    if text.is_empty() {
        None
    } else {
        Some(text.to_string_lossy().into_owned())
    }
}

fn steam_id_to_sys(id: SteamId) -> sys::CSteamID {
    sys::CSteamID {
        m_steamid: sys::CSteamID_SteamID_t {
            m_unAll64Bits: id.0,
        },
    }
}

fn steam_id_from_sys(id: sys::CSteamID) -> SteamId {
    // SAFETY: every member of the union shares the 64-bit view, which is the one read here
    SteamId(unsafe { id.m_steamid.m_unAll64Bits })
}

/// Copies the text of a chat message that the SDK wrote into `buffer`. `len` is the length the SDK returned; a
/// negative length gives `None`, and the text is cut at the first NUL byte.
fn chat_text(buffer: &[u8], len: i32) -> Option<String> {
    let len = usize::try_from(len).ok()?.min(buffer.len());
    let bytes = &buffer[..len];
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    Some(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

/// Maps the SDK chat entry type to [`ChatEntryType`]. Values that do not fit a `u8` are `Invalid`.
fn chat_entry_type_from_raw(raw: u32) -> ChatEntryType {
    u8::try_from(raw)
        .map(ChatEntryType::from)
        .unwrap_or(ChatEntryType::Invalid)
}
