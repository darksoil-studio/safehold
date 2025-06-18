use hdk::prelude::*;

#[derive(PartialEq, Eq, Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct EncryptedData(pub XSalsa20Poly1305EncryptedData);

pub fn to_bytes(encrypted_data: XSalsa20Poly1305EncryptedData) -> ExternResult<Vec<u8>> {
    let bytes =
        SerializedBytes::try_from(EncryptedData(encrypted_data)).map_err(|err| wasm_error!(err))?;

    let unsafe_bytes = UnsafeBytes::from(bytes);

    Ok(unsafe_bytes.into())
}
