use hc_zome_traits::*;
use hdk::prelude::*;
use locker_service_trait::*;
use locker_types::*;

fn def() {
    let dna_def: DnaDef = include_dna_def::include_dna_def!(
        "/home/guillem/projects/darksoil/locker/dnas/locker_service/workdir/locker.dna"
    );
}

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
        LOCKER_SERVICE_HASH,
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
    LockerService(LockerGateway),
}

pub struct LockerGateway;

#[implement_zome_trait_as_externs]
impl LockerService for LockerGateway {
    fn store_messages(messages: Vec<MessageWithProvenance>) -> ExternResult<()> {
        let sender = call_info()?.provenance;

        for message in &messages {
            if message.provenance.ne(&sender) {
                return Err(wasm_error!(
                    "Message provenance is not the caller of store_messages."
                ));
            }
        }

        let response = call(
            CallTargetCell::OtherRole(RoleName::from("locker")),
            ZomeName::from("locker"),
            FunctionName::from("create_messages"),
            None,
            messages,
        )?;
        let ZomeCallResponse::Ok(_) = response else {
            return Err(wasm_error!("Failed to store message: {response:?}"));
        };
        Ok(())
    }

    fn get_messages(_: ()) -> ExternResult<Vec<MessageOutput>> {
        let agent = call_info()?.provenance;
        let response = call(
            CallTargetCell::OtherRole(RoleName::from("locker")),
            ZomeName::from("locker"),
            FunctionName::from("get_messages_for_recipient"),
            None,
            agent,
        )?;
        let ZomeCallResponse::Ok(result) = response else {
            return Err(wasm_error!("Failed to get messages: {:?}"));
        };
        let messages: Vec<MessageOutput> = result.decode().map_err(|err| wasm_error!("{}", err))?;
        Ok(messages)
    }
}
