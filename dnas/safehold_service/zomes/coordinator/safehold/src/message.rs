use hdk::prelude::*;
use safehold_integrity::*;
use safehold_service_trait::*;

use crate::utils::{create_link_relaxed, create_relaxed, delete_link_relaxed, ensure_relaxed};

fn agent_path(agent: AgentPubKey) -> ExternResult<TypedPath> {
    Path::from(format!("all_agents.{}", agent)).typed(LinkTypes::AgentsPath)
}

#[hdk_extern]
pub fn create_messages(inputs: Vec<MessageWithProvenance>) -> ExternResult<()> {
    for input in inputs {
        create_message(input)?;
    }

    Ok(())
}

#[hdk_extern]
pub fn create_message(message: MessageWithProvenance) -> ExternResult<EntryHash> {
    info!("Creating message.");

    let message_hash = hash_entry(&message)?;

    let None = get(message_hash.clone(), GetOptions::default())? else {
        return Ok(message_hash);
    };

    create_relaxed(EntryTypes::Message(message.clone()))?;

    for (agent, contents) in message.message.recipients.clone() {
        let path = agent_path(agent)?;

        ensure_relaxed(&path)?;

        create_link_relaxed(
            path.path_entry_hash()?,
            message_hash.clone(),
            LinkTypes::RecipientToMessages,
            contents,
        )?;
    }
    Ok(message_hash)
}

#[hdk_extern]
pub fn get_messages_for_recipient(recipient: AgentPubKey) -> ExternResult<Vec<MessageOutput>> {
    let path = agent_path(recipient)?;
    let links = get_links(
        GetLinksInputBuilder::try_new(path.path_entry_hash()?, LinkTypes::RecipientToMessages)?
            .build(),
    )?;

    for link in &links {
        delete_link_relaxed(link.create_link_hash.clone())?;
    }

    let inputs = links
        .iter()
        .filter_map(|l| l.target.clone().into_entry_hash())
        .map(|entry_hash| GetInput::new(entry_hash.into(), GetOptions::default()))
        .collect();

    let records = HDK.with(|hdk| hdk.borrow().get(inputs))?;

    let messages: Vec<MessageOutput> = records
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
        .zip(links)
        .map(|(message, link)| MessageOutput {
            provenance: message.provenance,
            message_contents: message.message.contents,
            agent_specific_contents: link.tag.0,
        })
        .collect();

    if !messages.is_empty() {
        info!("Delived {} messages.", messages.len());
    }

    Ok(messages)
}
