use hdi::prelude::*;
pub use locker_types::MessageWithProvenance;

pub fn validate_create_message(
    _action: EntryCreationAction,
    message: MessageWithProvenance,
) -> ExternResult<ValidateCallbackResult> {
    let Ok(true) = verify_signature(
        message.sender.clone(),
        message.signature.clone(),
        &message.contents,
    ) else {
        return Ok(ValidateCallbackResult::Invalid(String::from(
            "Invalid signature",
        )));
    };
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_message(
    _action: Update,
    _message: MessageWithProvenance,
    _original_action: EntryCreationAction,
    _original_message: MessageWithProvenance,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "Messages cannot be updated".to_string(),
    ))
}

pub fn validate_delete_message(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_message: MessageWithProvenance,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "Messages cannot be deleted".to_string(),
    ))
}

pub fn validate_create_link_recipient_to_messages(
    _action: CreateLink,
    _base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    let entry_hash = target_address
        .into_entry_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No action hash associated with link".to_string()
        )))?;
    let entry = must_get_entry(entry_hash)?;
    let _message = crate::MessageWithProvenance::try_from(entry.content)?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_recipient_to_messages(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}
