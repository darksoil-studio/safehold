use hdk::prelude::*;
use safehold_integrity::LinkTypes;
use safehold_types::MessageWithProvenance;

#[hdk_extern]
pub fn export_undeleted_messages() -> ExternResult<Vec<MessageWithProvenance>> {
    let path = Path::from(format!("all_agents")).typed(LinkTypes::AgentsPath)?;

    let children = path.children()?;

    let get_links_input: Vec<GetLinksInput> = children
        .into_iter()
        .map(|link| LinkQuery::try_new(link.target, LinkTypes::RecipientToMessages))
        .collect::<ExternResult<Vec<LinkQuery>>>()?
        .into_iter()
        .map(|query| GetLinksInput::from_query(query, GetOptions::network()))
        .collect();

    let links = HDK.with(|h| h.borrow().get_links(get_links_input))?;

    let undeleted_messages_entry_hashes: BTreeSet<EntryHash> = links
        .into_iter()
        .flatten()
        .filter_map(|l| l.target.into_entry_hash())
        .collect();

    let get_inputs: Vec<GetInput> = undeleted_messages_entry_hashes
        .into_iter()
        .map(|e| GetInput::new(e.into(), GetOptions::default()))
        .collect();

    let records = HDK.with(|h| h.borrow().get(get_inputs))?;

    let messages: Vec<MessageWithProvenance> = records
        .into_iter()
        .filter_map(|r| r)
        .filter_map(|record| {
            let Some(entry) = record.entry().as_option() else {
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
