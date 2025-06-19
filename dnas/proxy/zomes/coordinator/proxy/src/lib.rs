use hdk::prelude::*;
use locker_types::ProxiedCall;
use proxy_integrity::*;

#[hdk_extern]
pub fn create_proxied_role(proxied_role: String) -> ExternResult<Record> {
    let proxied_role_hash = create_entry(&EntryTypes::ProxiedRole(ProxiedRole { proxied_role }))?;
    let record = get(proxied_role_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly created ProxiedRole".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn query_proxied_role() -> ExternResult<Option<String>> {
    let records = query(
        ChainQueryFilter::new()
            .include_entries(true)
            .entry_type(UnitEntryTypes::ProxiedRole.try_into()?),
    )?;

    let Some(last_record) = records.into_iter().max_by_key(|r| r.action().timestamp()) else {
        return Ok(None);
    };
    let Some(entry) = last_record.entry().as_option() else {
        return Ok(None);
    };
    let Ok(role) = ProxiedRole::try_from(entry) else {
        return Ok(None);
    };
    Ok(Some(role.proxied_role))
}

#[hdk_extern]
pub fn proxied_call(input: ProxiedCall) -> ExternResult<ExternIO> {
    let Some(role) = query_proxied_role(())? else {
        return Err(wasm_error!("No proxied role found"));
    };

    let response = call(
        CallTargetCell::OtherRole(role),
        input.zome_name,
        input.fn_name,
        None,
        input.payload,
    )?;
    let ZomeCallResponse::Ok(result) = response else {
        return Err(wasm_error!("Failed to make proxied call: {response:?}"));
    };

    let result: ExternIO = result.decode().map_err(|err| wasm_error!("{}", err))?;
    Ok(result)
}
