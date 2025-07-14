use hdi::prelude::*;
use safehold_types::MessageContents;

pub type MessageId = Vec<u8>;

#[derive(Clone)]
#[hdk_entry_helper]
pub struct Chunk {
    pub provenance: AgentPubKey,
    pub message_id: MessageId,
    pub chunk_index: usize,
    pub total_chunk_number: usize,
    pub contents: MessageContents,
}

pub fn validate_create_chunk(
    _action: EntryCreationAction,
    chunk: Chunk,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_chunk(
    _action: Update,
    _chunk: Chunk,
    _original_action: EntryCreationAction,
    _original_chunk: Chunk,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "chunks cannot be updated".to_string(),
    ))
}

pub fn validate_delete_chunk(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_chunk: Chunk,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "chunks cannot be deleted".to_string(),
    ))
}
