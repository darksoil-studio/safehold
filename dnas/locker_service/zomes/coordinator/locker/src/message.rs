use hdk::prelude::*;
use locker_integrity::*;

#[hdk_extern]
pub fn create_message(message: MessageWithProvenance) -> ExternResult<Record> {
    let message_hash = hash_entry(&message)?;
    create_entry(&EntryTypes::Message(message.clone()))?;
    for base in message.message.recipients.clone() {
        create_link(
            base,
            message_hash.clone(),
            LinkTypes::RecipientToMessages,
            (),
        )?;
    }
    let record = get(message_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly created Message".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn get_messages_for_recipient(
    recipient: AgentPubKey,
) -> ExternResult<Vec<MessageWithProvenance>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(recipient, LinkTypes::RecipientToMessages)?.build(),
    )?;

    for link in &links {
        delete_link(link.create_link_hash.clone())?;
    }

    let inputs = links
        .into_iter()
        .filter_map(|l| l.target.into_entry_hash())
        .map(|entry_hash| GetInput::new(entry_hash.into(), GetOptions::default()))
        .collect();

    let records = HDK.with(|hdk| hdk.borrow().get(inputs))?;

    let messages = records
        .into_iter()
        .filter_map(|r| r)
        .filter_map(|r| {
            let Some(entry) = r.entry().as_option() else {
                return None;
            };
            let Ok(message) = MessageWithProvenance::try_from(entry) else {
                return None;
            };
            Some(message)
        })
        .collect();

    Ok(messages)
}
