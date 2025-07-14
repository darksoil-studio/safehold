use std::collections::BTreeMap;

use encrypted_messages_integrity::*;
use hdk::prelude::*;

pub fn query_pending_chunks(
) -> ExternResult<BTreeMap<(MessageId, AgentPubKey), Vec<(ActionHash, Chunk)>>> {
    let chunks = query_pending_chunks_entries()?;

    let mut pending_chunks: BTreeMap<(MessageId, AgentPubKey), Vec<(ActionHash, Chunk)>> =
        BTreeMap::new();

    for chunk in chunks {
        pending_chunks
            .entry((chunk.1.message_id.clone(), chunk.1.provenance.clone()))
            .or_insert(Default::default())
            .push(chunk);
    }

    Ok(pending_chunks)
}

pub fn query_pending_chunks_entries() -> ExternResult<Vec<(ActionHash, Chunk)>> {
    let records = query(
        ChainQueryFilter::new()
            .entry_type(UnitEntryTypes::Chunk.try_into()?)
            .action_type(ActionType::Create),
    )?;
    let delete_records = query(ChainQueryFilter::new().action_type(ActionType::Delete))?;

    let deleted_entries: BTreeSet<EntryHash> = delete_records
        .into_iter()
        .filter_map(|r| match r.action() {
            Action::Delete(d) => Some(d.clone()),
            _ => None,
        })
        .map(|d| d.deletes_entry_address)
        .collect();

    let undeleted_entry_hashes: BTreeSet<EntryHash> = records
        .into_iter()
        .filter_map(|r| match r.action() {
            Action::Create(c) => Some(c.clone()),
            _ => None,
        })
        .map(|c| c.entry_hash)
        .filter(|entry_hash| !deleted_entries.contains(entry_hash))
        .collect();

    let get_inputs: Vec<GetInput> = undeleted_entry_hashes
        .into_iter()
        .map(|entry_hash| GetInput::new(entry_hash.into(), GetOptions::local()))
        .collect();

    let chunks: Vec<(ActionHash, Chunk)> = HDK
        .with(|hdk| hdk.borrow().get(get_inputs))?
        .into_iter()
        .filter_map(|r| r)
        .filter_map(|r| {
            let action_hash = r.action_address().clone();
            let Some(entry) = r.entry.into_option() else {
                return None;
            };
            let Ok(chunk) = Chunk::try_from(entry) else {
                return None;
            };
            Some((action_hash, chunk))
        })
        .collect();

    Ok(chunks)
}
