//! Argument validation never initializes the real Steam API.
use crabby_ffi::*;
#[test]
fn steam_message_and_lobby_arguments_fail_before_initialization() {
    let mut endpoint = std::ptr::null_mut();
    let mut lobby = std::ptr::null_mut();
    // SAFETY: writable outputs and null handles are permitted by these APIs.
    unsafe {
        assert_eq!(
            crabby_net_messages_steam_host(480, 0, &mut endpoint),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert!(endpoint.is_null());
        assert_eq!(
            crabby_net_messages_steam_join(480, 0, &mut endpoint),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_steam_lobby_create(480, 3, 4, &mut lobby),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_steam_lobby_create(480, 0, 251, &mut lobby),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_steam_lobby_join(480, 0, &mut lobby),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert!(lobby.is_null());
        assert_ne!(crabby_steam_lobby_poll(lobby), CrabbyStatus::CRABBY_OK);
        assert_ne!(crabby_steam_lobby_leave(lobby), CrabbyStatus::CRABBY_OK);
        assert_ne!(crabby_steam_lobby_invite(lobby), CrabbyStatus::CRABBY_OK);
        crabby_steam_lobby_free(lobby);
    }
}
