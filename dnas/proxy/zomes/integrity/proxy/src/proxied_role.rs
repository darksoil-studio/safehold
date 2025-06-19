use hdi::prelude::*;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct ProxiedRole {
    pub proxied_role: String,
}

pub fn validate_create_proxied_role(
    _action: EntryCreationAction,
    _proxied_role: ProxiedRole,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_proxied_role(
    _action: Update,
    _proxied_role: ProxiedRole,
    _original_action: EntryCreationAction,
    _original_proxied_role: ProxiedRole,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "Proxied Roles cannot be updated".to_string(),
    ))
}

pub fn validate_delete_proxied_role(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_proxied_role: ProxiedRole,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "Proxied Roles cannot be deleted".to_string(),
    ))
}
