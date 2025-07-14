use hc_zome_traits::*;
use hdk::prelude::*;
use safehold_service_trait::*;
use safehold_types::*;

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    let mut fns: BTreeSet<GrantedFunction> = BTreeSet::new();
    fns.insert((zome_info()?.name, FunctionName::from("get_messages")));
    fns.insert((zome_info()?.name, FunctionName::from("store_messages")));
    let functions = GrantedFunctions::Listed(fns);
    let cap_grant = ZomeCallCapGrant {
        tag: String::from("store_and_get_messages"),
        access: CapAccess::Unrestricted,
        functions,
    };
    create_cap_grant(cap_grant)?;

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("service_providers"),
        "announce_as_provider".into(),
        None,
        SAFEHOLD_SERVICE_HASH,
    )?;
    let ZomeCallResponse::Ok(_) = response else {
        return Ok(InitCallbackResult::Fail(format!(
            "Failed to announce as provider: {response:?}"
        )));
    };

    Ok(InitCallbackResult::Pass)
}

#[implemented_zome_traits]
pub enum ZomeTraits {
    SafeholdService(SafeholdGateway),
}

pub struct SafeholdGateway;

#[implement_zome_trait_as_externs]
impl SafeholdService for SafeholdGateway {
    fn store_messages(messages: Vec<MessageWithProvenance>) -> ExternResult<()> {
        let sender = call_info()?.provenance;

        for message in &messages {
            if message.provenance.ne(&sender) {
                return Err(wasm_error!(
                    "Message provenance is not the caller of store_messages."
                ));
            }
        }

        let proxied_call = ProxiedCall {
            zome_name: ZomeName::from("safehold"),
            fn_name: FunctionName::from("create_messages"),
            payload: ExternIO::encode(messages).map_err(|err| wasm_error!(err))?,
        };

        let response = call(
            CallTargetCell::OtherRole(RoleName::from("proxy")),
            ZomeName::from("proxy"),
            FunctionName::from("proxied_call"),
            None,
            proxied_call,
        )?;
        let ZomeCallResponse::Ok(_) = response else {
            return Err(wasm_error!("Failed to store message: {response:?}"));
        };
        Ok(())
    }

    fn get_messages(_: ()) -> ExternResult<Vec<MessageOutput>> {
        let agent = call_info()?.provenance;

        let proxied_call = ProxiedCall {
            zome_name: ZomeName::from("safehold"),
            fn_name: FunctionName::from("get_messages_for_recipient"),
            payload: ExternIO::encode(agent).map_err(|err| wasm_error!(err))?,
        };

        let response = call(
            CallTargetCell::OtherRole(RoleName::from("proxy")),
            ZomeName::from("proxy"),
            FunctionName::from("proxied_call"),
            None,
            proxied_call,
        )?;
        let ZomeCallResponse::Ok(result) = response else {
            return Err(wasm_error!("Failed to get messages: {:?}"));
        };
        let result: ExternIO = result.decode().map_err(|err| wasm_error!("{}", err))?;
        let messages: Vec<MessageOutput> = result.decode().map_err(|err| wasm_error!("{}", err))?;
        Ok(messages)
    }
}
