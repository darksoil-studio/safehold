use hdk::prelude::*;
use locker_integrity::*;
use locker_service_trait::*;
use locker_types::*;

#[hdk_extern]
pub fn create_messages(inputs: Vec<CreateMessageInput>) -> ExternResult<()> {
    for input in inputs {
        create_message(input)?;
    }

    Ok(())
}

#[hdk_extern]
pub fn create_message(input: CreateMessageInput) -> ExternResult<Record> {
    let message = input.message;
    let message_hash = hash_entry(&message)?;
    create_entry(&EntryTypes::Message(message.clone()))?;
    for (agent, contents) in input.recipients.clone() {
        create_link(
            agent,
            message_hash.clone(),
            LinkTypes::RecipientToMessages,
            contents,
        )?;
    }
    let record = get(message_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly created Message".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn get_messages_for_recipient(recipient: AgentPubKey) -> ExternResult<Vec<MessageOutput>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(recipient, LinkTypes::RecipientToMessages)?.build(),
    )?;

    for link in &links {
        delete_link(link.create_link_hash.clone())?;
    }

    let inputs = links
        .iter()
        .filter_map(|l| l.target.clone().into_entry_hash())
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
            let Ok(message) = Message::try_from(entry) else {
                return None;
            };
            Some(message)
        })
        .zip(links)
        .map(|(message, link)| MessageOutput {
            message,
            agent_specific_contents: link.tag.0,
        })
        .collect();

    Ok(messages)
}
