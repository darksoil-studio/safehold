use holochain_types::prelude::SerializedBytes;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};
use tokio::runtime::Runtime;

#[proc_macro]
pub fn include_dna_def(item: TokenStream) -> TokenStream {
    let rt = Runtime::new().unwrap();

    rt.handle().block_on(async move {
        let path = parse_macro_input!(item as LitStr);

        let bytes = std::fs::read(path.value()).expect("Failed to read dna file");
        let bundle = holochain_types::prelude::DnaBundle::decode(bytes.as_slice())
            .expect("Failed to decode dna bundle");
        let (file, _) = bundle
            .to_dna_file()
            .await
            .expect("Failed to convert bundle to dna file");
        let dna_def = file.dna_def();

        let bytes = SerializedBytes::try_from(dna_def).expect("Failed to serialize");
        let bytes = bytes.bytes().iter().map(|&c| c as char).collect::<String>();

        quote! {
            DnaDef::try_from(SerializedBytes::from(UnsafeBytes::from(#bytes.chars().into_iter().map(|c| c as u8).collect::<Vec<u8>>()))).expect("Failed to deserialize")
        }
        .into()
    })
}
