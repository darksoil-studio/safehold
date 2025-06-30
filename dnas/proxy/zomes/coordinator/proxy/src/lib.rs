use hdk::prelude::*;
use safehold_types::ProxiedCall;
use proxy_integrity::*;

#[hdk_extern]
pub fn create_proxied_dna(proxied_dna: DnaHash) -> ExternResult<Record> {
    let proxied_role_hash = create_entry(&EntryTypes::ProxiedDna(ProxiedDna { proxied_dna }))?;
    let record = get(proxied_role_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly created ProxiedRole".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn query_proxied_dna() -> ExternResult<Option<DnaHash>> {
    let records = query(
        ChainQueryFilter::new()
            .include_entries(true)
            .entry_type(UnitEntryTypes::ProxiedDna.try_into()?),
    )?;

    let Some(last_record) = records.into_iter().max_by_key(|r| r.action().timestamp()) else {
        return Ok(None);
    };
    let Some(entry) = last_record.entry().as_option() else {
        return Ok(None);
    };
    let Ok(role) = ProxiedDna::try_from(entry) else {
        return Ok(None);
    };
    Ok(Some(role.proxied_dna))
}

#[hdk_extern]
pub fn proxied_call(input: ProxiedCall) -> ExternResult<ExternIO> {
    let Some(dna_hash) = query_proxied_dna(())? else {
        return Err(wasm_error!("No proxied role found"));
    };

    let cell_id = CellId::new(dna_hash, agent_info()?.agent_initial_pubkey);

    let response = HDK.with(|h| {
        h.borrow().call(vec![Call::new(
            CallTarget::ConductorCell(CallTargetCell::OtherCell(cell_id)),
            input.zome_name,
            input.fn_name,
            None,
            input.payload,
        )])
    })?;
    let Some(ZomeCallResponse::Ok(result)) = response.get(0) else {
        return Err(wasm_error!("Failed to make proxied call: {response:?}"));
    };

    // let result: ExternIO = result.decode().map_err(|err| wasm_error!("{}", err))?;
    Ok(result.clone())
}
