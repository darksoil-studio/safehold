use encrypted_messages_integrity::EntryTypes;
use hdk::prelude::*;

#[derive(PartialEq, Eq, Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct EncryptedData(pub XSalsa20Poly1305EncryptedData);

pub fn to_bytes(encrypted_data: XSalsa20Poly1305EncryptedData) -> ExternResult<Vec<u8>> {
    let bytes =
        SerializedBytes::try_from(EncryptedData(encrypted_data)).map_err(|err| wasm_error!(err))?;

    let unsafe_bytes = UnsafeBytes::from(bytes);

    Ok(unsafe_bytes.into())
}

pub fn from_bytes(bytes: Vec<u8>) -> ExternResult<XSalsa20Poly1305EncryptedData> {
    let bytes = SerializedBytes::from(UnsafeBytes::from(bytes));

    let data = EncryptedData::try_from(bytes).map_err(|err| wasm_error!(err))?;
    Ok(data.0)
}

///Allow other processes to get commited to source chain before commiting the commit
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
