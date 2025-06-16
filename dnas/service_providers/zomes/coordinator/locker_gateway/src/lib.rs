use hdk::prelude::*;
use locker_service_trait::locker_SERVICE_HASH;

mod locker_service;

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    let mut fns: BTreeSet<GrantedFunction> = BTreeSet::new();
    fns.insert((zome_info()?.name, FunctionName::from("register_fcm_token")));
    fns.insert((
        zome_info()?.name,
        FunctionName::from("send_push_notification"),
    ));
    let functions = GrantedFunctions::Listed(fns);
    let cap_grant = ZomeCallCapGrant {
        tag: String::from("send_push_notification"),
        access: CapAccess::Unrestricted,
        functions,
    };
    create_cap_grant(cap_grant)?;

    let response = call(
        CallTargetCell::Local,
        ZomeName::from("service_providers"),
        "announce_as_provider".into(),
        None,
        locker_SERVICE_HASH,
    )?;
    let ZomeCallResponse::Ok(_) = response else {
        return Ok(InitCallbackResult::Fail(format!(
            "Failed to announce as provider: {response:?}"
        )));
    };

    Ok(InitCallbackResult::Pass)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Signal {}

#[hdk_extern(infallible)]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    for action in committed_actions {
        if let Err(err) = signal_action(action) {
            error!("Error signaling new action: {:?}", err);
        }
    }
}
fn signal_action(action: SignedActionHashed) -> ExternResult<()> {
    Ok(())
}
