use std::num::NonZeroUsize;
use std::str::FromStr;
use dash_sdk::{mock::provider::GrpcContextProvider, SdkBuilder};
use dash_sdk::platform::{DataContract, Fetch, Identifier};
use dash_sdk::sdk::AddressList;
use drive::dpp::platform_value::string_encoding::Encoding::Base58;

#[tokio::main]
async fn main() {
    let data_contract_identifier: [u8; 32] = Identifier::from_string("2twstHkD3uYEogneYppHDCfnnfKxDk6YeJrKt3qNwtcW", Base58)
        .expect("Could not parse data contract identifier")
        .into();

    let server_address: String = String::from("127.0.0.1");
    let core_port: u16 = 19998;
    let platform_port: u16 = 1443;
    let core_user: String = String::from("dashmate");
    let core_password: String = String::from("password");

    let context_provider = GrpcContextProvider::new(
        None,
        &server_address,
        core_port,
        &core_user,
        &core_password,
        NonZeroUsize::new(100).expect("data contracts cache size"),
        NonZeroUsize::new(100).expect("quorum public keys cache size"),
    )
        .expect("context provider");

    let uri = http::Uri::from_str(&format!(
        "http://{}:{}",
        &server_address, &platform_port
    ))
        .expect("parse uri");

    let mut sdk = SdkBuilder::new(AddressList::from_iter([uri]))
        .build()
        .expect("cannot build sdk");

    context_provider.set_sdk(Some(sdk.clone()));

    sdk.set_context_provider(context_provider);

    let id = Identifier::from_bytes(&data_contract_identifier).expect("parse data contract id");

    let contract: Option<DataContract> =
        DataContract::fetch(&sdk, id).await.expect("fetch identity");

    match contract {
        None => {
            println!("No contract found")
        }
        Some(_) => {
            println!("Contract is there")
        }
    }
}