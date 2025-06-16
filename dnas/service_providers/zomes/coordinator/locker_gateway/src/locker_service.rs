use hc_zome_traits::{implement_zome_trait_as_externs, implemented_zome_traits};
use hdk::prelude::*;
use locker_service_trait::{
    PushNotificationsService, RegisterFcmTokenInput, SendPushNotificationToAgentInput,
};
use locker_types::*;

#[implemented_zome_traits]
pub enum ZomeTraits {
    PushNotifications(PushNotificationsGateway),
}

pub struct PushNotificationsGateway;

#[implement_zome_trait_as_externs]
impl PushNotificationsService for PushNotificationsGateway {
    fn register_fcm_token(input: RegisterFcmTokenInput) -> ExternResult<()> {
        let agent = call_info()?.provenance;
        let response = call(
            CallTargetCell::OtherRole(RoleName::from("locker_service")),
            ZomeName::from("locker_service"),
            FunctionName::from("register_fcm_token_for_agent"),
            None,
            RegisterFcmTokenForAgentInput {
                fcm_project_id: input.fcm_project_id,
                token: input.token,
                agent,
            },
        )?;
        let ZomeCallResponse::Ok(_) = response else {
            return Err(wasm_error!("Failed to register fcm token: {response:?}"));
        };
        Ok(())
    }

    fn send_push_notification(input: SendPushNotificationToAgentInput) -> ExternResult<()> {
        let agent = call_info()?.provenance;
        let response = call(
            CallTargetCell::OtherRole(RoleName::from("locker_service")),
            ZomeName::from("locker_service"),
            FunctionName::from("send_push_notification_to_agent"),
            None,
            SendPushNotificationToAgentWithProvenanceInput {
                provenance: agent,
                agent: input.agent,
                notification: input.notification,
            },
        )?;
        let ZomeCallResponse::Ok(_) = response else {
            return Err(wasm_error!("Failed to register fcm token: {response:?}"));
        };
        Ok(())
    }
}
