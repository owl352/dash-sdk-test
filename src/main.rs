use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Context;
use dash_sdk::{mock::provider::GrpcContextProvider, SdkBuilder};
use dash_sdk::dpp::data_contract::document_type::DocumentType;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::platform_value;
use dash_sdk::dpp::util::entropy_generator::EntropyGenerator;
use dash_sdk::platform::{DataContract, Document, Fetch, Identifier, Identity};
use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::sdk::AddressList;
use dpp::dashcore::{PrivateKey};
use dpp::document::{DocumentV0, INITIAL_REVISION};
use drive::dpp::platform_value::string_encoding::Encoding::Base58;
use drive::dpp::version::PlatformVersion;
use getrandom::getrandom;
use simple_signer::signer::SimpleSigner;

use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};

use dotenv::dotenv;
use std::env;

pub struct MyDefaultEntropyGenerator;

impl EntropyGenerator for MyDefaultEntropyGenerator {
    fn generate(&self) -> anyhow::Result<[u8; 32]> {
        let mut buffer = [0u8; 32];
        getrandom(&mut buffer).context("generating entropy failed")?;
        Ok(buffer)
    }
}


#[tokio::main]
async fn main() {
    dotenv().ok();

    let data_contract_identifier: [u8; 32] = Identifier::from_string("2twstHkD3uYEogneYppHDCfnnfKxDk6YeJrKt3qNwtcW", Base58)
        .expect("Could not parse data contract identifier")
        .into();
    let identity_identifier: [u8; 32] = Identifier::from_string("9Upw4Yd8FmL6XvjTpAHguqWg227KkfRbmbhnfZFV7UuB", Base58)
        .expect("Could not parse identity identifier")
        .into();


    let private_key = PrivateKey::from_wif("cQ9xWG9f2gQjJ2uxqKDFFy7crSpziY4oADnPQfvGyQq3coKSo9XV")
        .expect("Could not parse pk");

    let data_contract_schema = platform_value!({
      "Project": {
        "type": "object",
        "properties": {
          "name": {
            "position": 0,
            "type": "string",
            "description": "Project name",
            "maxLength": 63
          },
          "description": {
            "position": 1,
            "type": "string",
            "description": "Project description",
            "maxLength": 1000
          },
          "url": {
            "position": 2,
            "type": "string",
            "description": "Project URL",
            "maxLength": 255
          }
        },
        "required": [
          "name",
          "description",
          "url",
          "$createdAt",
          "$updatedAt"
        ],
        "additionalProperties": false
      },
      "Tasks": {
        "type": "object",
        "properties": {
          "title": {
            "position": 0,
            "type": "string",
            "description": "Task title",
            "maxLength": 63
          },
          "description": {
            "position": 1,
            "type": "string",
            "description": "Task description",
            "maxLength": 1000
          },
          "url": {
            "position": 2,
            "type": "string",
            "description": "Task URL",
            "maxLength": 255
          },
          "assignee": {
            "position": 3,
            "type": "array",
            "description": "Task assignee executor",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32
          },
          "projectId": {
            "position": 4,
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32
          },
          "status": {
            "position": 5,
            "type": "string",
            "description": "Task status",
            "enum": [
              "pending",
              "in_progress",
              "completed",
              "cancelled",
              "paid"
            ]
          }
        },
        "required": [
          "title",
          "projectId",
          "$createdAt",
          "$updatedAt"
        ],
        "additionalProperties": false
      },
      "Claim": {
        "type": "object",
        "properties": {
          "taskId": {
            "position": 0,
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32
          },
          "amountCredits": {
            "position": 1,
            "type": "number"
          },
          "amountUSD": {
            "position": 2,
            "type": "number"
          }
        },
        "required": [
          "$createdAt",
          "$updatedAt",
          "taskId",
          "amountCredits",
          "amountUSD"
        ],
        "additionalProperties": false
      }
    });

    let arr = [
        81,
        4,
        39,
        130,
        0,
        31,
        33,
        150,
        119,
        144,
        62,
        21,
        9,
        138,
        172,
        48,
        113,
        169,
        210,
        246,
        113,
        194,
        162,
        177,
        44,
        78,
        160,
        140,
        180,
        214,
        61,
        237
    ];

    let document_properties = platform_value!(
     {
      "taskId": bytes::Bytes::copy_from_slice(&arr),
      "amountCredits": 20,
      "amountUSD": 500
    });

    let document_type_name = "Claim";

    let server_address = env::var("SERVER_ADDRESS").expect("SERVER_ADDRESS not set");
    let core_port: u16 = env::var("CORE_PORT").expect("CORE_PORT not set").parse().unwrap();
    let platform_port: u16 = env::var("PLATFORM_PORT").expect("PLATFORM_PORT not set").parse().unwrap();
    let core_user = env::var("CORE_USER").expect("CORE_USER not set");
    let core_password = env::var("CORE_PASSWORD").expect("CORE_PASSWORD not set");

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

    let identity_id = Identifier::from(identity_identifier);
    let identity = Identity::fetch_by_identifier(&sdk, identity_id).await.unwrap().expect("Identity not found");

    let data_contract_identifier = Identifier::from_bytes(&data_contract_identifier).expect("parse data contract id");

    let contract: DataContract =
        DataContract::fetch(&sdk, data_contract_identifier).await.expect("fetch identity").expect("Data contract not found");

    let now = SystemTime::now();
    let now_seconds = now
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let mut std_rng = StdRng::from_entropy();
    let document_state_transition_entropy: [u8; 32] = std_rng.gen();

    let document_id = Document::generate_document_id_v0(
        &data_contract_identifier,
        &identity_id,
        &document_type_name,
        &document_state_transition_entropy,
    );

    let document: Document = Document::V0(DocumentV0 {
        id: document_id,
        properties: document_properties.into_btree_string_map().unwrap(),
        owner_id: identity_id, //
        revision: Some(INITIAL_REVISION),
        created_at: Some(now_seconds),
        updated_at: Some(now_seconds),
        transferred_at: None,
        created_at_block_height: None,
        updated_at_block_height: None,
        transferred_at_block_height: None,
        created_at_core_block_height: None,
        updated_at_core_block_height: None,
        transferred_at_core_block_height: None,
    });

    let new_document_type = DocumentType::try_from_schema(
        data_contract_identifier,
        document_type_name,
        data_contract_schema,
        None,
        false,
        false,
        false,
        false,
        &mut Vec::new(),
        PlatformVersion::latest(),
    )
        .expect("failed to create new document type");

    let identity_public_key = identity.get_public_key_by_id(1)
        .expect("Could not match identity public key");


    let mut signer = SimpleSigner::default();

    signer.add_key(identity_public_key.clone(), private_key.to_bytes().clone());
        // .private_keys
        // .insert(identity_public_key.clone(), private_key.to_bytes());

    let data_contract_arc = Arc::new(contract.clone());

    let new_document = document.put_to_platform_and_wait_for_response(
        &sdk,
        new_document_type,
        document_state_transition_entropy,
        identity_public_key.clone(),
        data_contract_arc,
        &signer,
    ).await.expect("There was a error pushing the document");

    println!("OK")
}
