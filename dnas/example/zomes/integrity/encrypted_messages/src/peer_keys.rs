use hdi::prelude::*;

#[derive(Clone)]
#[hdk_entry_helper]
pub struct PeerKeys {
    pub peer: AgentPubKey,
    pub my_current_key: Option<X25519PubKey>,
    pub their_current_key: Option<X25519PubKey>,
}

pub fn validate_create_peer_keys(
    _action: EntryCreationAction,
    peer_keys: PeerKeys,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_peer_keys(
    _action: Update,
    _peer_keys: PeerKeys,
    _original_action: EntryCreationAction,
    _original_peer_keys: PeerKeys,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "peer_keyss cannot be updated".to_string(),
    ))
}

pub fn validate_delete_peer_keys(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_peer_keys: PeerKeys,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "peer_keyss cannot be deleted".to_string(),
    ))
}
