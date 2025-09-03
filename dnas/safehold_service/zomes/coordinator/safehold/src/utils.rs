use hdi::hash_path::path::root_hash;
use hdk::prelude::*;
use safehold_integrity::EntryTypes;

pub fn ensure_relaxed(path: &TypedPath) -> ExternResult<()> {
    if !path.exists()? {
        if path.is_root() {
            create_link_relaxed(
                root_hash()?,
                path.path_entry_hash()?,
                path.link_type,
                path.make_tag()?,
            )?;
        } else if let Some(parent) = path.parent() {
            ensure_relaxed(&parent)?;
            create_link_relaxed(
                parent.path_entry_hash()?,
                path.path_entry_hash()?,
                path.link_type,
                path.make_tag()?,
            )?;
        }
    }
    Ok(())
}

pub fn create_relaxed(entry_type: EntryTypes) -> ExternResult<()> {
    HDK.with(|h| {
        let index = ScopedEntryDefIndex::try_from(&entry_type)?;
        let vis = EntryVisibility::from(&entry_type);
        let entry = Entry::try_from(entry_type)?;

        h.borrow().create(CreateInput::new(
            index,
            vis,
            entry,
            // This is used to test many conductors thrashing creates between
            // each other so we want to avoid retries that make the test take
            // a long time.
            ChainTopOrdering::Relaxed,
        ))
    })?;

    Ok(())
}

pub fn delete_link_relaxed(address: ActionHash) -> ExternResult<()> {
    HDK.with(|h| {
        h.borrow().delete_link(DeleteLinkInput::new(
            address,
            GetOptions::network(),
            ChainTopOrdering::Relaxed,
        ))
    })?;

    Ok(())
}

///Allowing other links to be created and commited before commiting link
pub fn create_link_relaxed<T, E>(
    base_address: impl Into<AnyLinkableHash>,
    target_address: impl Into<AnyLinkableHash>,
    link_type: T,
    tag: impl Into<LinkTag>,
) -> ExternResult<()>
where
    ScopedLinkType: TryFrom<T, Error = E>,
    WasmError: From<E>,
{
    let ScopedLinkType {
        zome_index,
        zome_type: link_type,
    } = link_type.try_into()?;
    HDK.with(|h| {
        h.borrow().create_link(CreateLinkInput::new(
            base_address.into(),
            target_address.into(),
            zome_index,
            link_type,
            tag.into(),
            ChainTopOrdering::Relaxed,
        ))
    })?;

    Ok(())
}
