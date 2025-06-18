use encrypted_messages_integrity::*;
use hdk::prelude::*;

pub fn query_all_peer_keys() -> ExternResult<Vec<(Record, PeerKeys)>> {
    let records = query(
        ChainQueryFilter::new()
            .include_entries(true)
            .entry_type(UnitEntryTypes::PeerKeys.try_into()?),
    )?;

    let peer_keys: Vec<(Record, PeerKeys)> = records
        .into_iter()
        .filter_map(|record| {
            let Some(entry) = record.entry().as_option() else {
                return None;
            };
            let Ok(peer_keys) = PeerKeys::try_from(entry) else {
                return None;
            };
            Some((record, peer_keys))
        })
        .collect();

    Ok(peer_keys)
}

pub fn query_peer_keys(recipient: &AgentPubKey) -> ExternResult<Option<PeerKeys>> {
    let peer_keys = query_all_peer_keys()?;

    let recipient_peer_keys: Vec<(Record, PeerKeys)> = peer_keys
        .into_iter()
        .filter(|(_, peer_keys)| peer_keys.peer.eq(recipient))
        .collect();

    let Some((_, last_peer_key)) = recipient_peer_keys
        .into_iter()
        .max_by_key(|(r, _)| r.action().timestamp())
    else {
        return Ok(None);
    };

    Ok(Some(last_peer_key))
}
