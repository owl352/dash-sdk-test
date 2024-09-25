use std::collections::HashSet;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Context;
use dash_sdk::{mock::provider::GrpcContextProvider, SdkBuilder};
use dash_sdk::dapi_client::mock::Key;
use dash_sdk::dpp::data_contract::document_type::DocumentType;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::platform_value;
use dash_sdk::dpp::util::entropy_generator::EntropyGenerator;
use dash_sdk::platform::{DataContract, Document, DocumentQuery, Fetch, Identifier, Identity};
use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::sdk::AddressList;
use dpp::dashcore::{Network, PrivateKey};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{DocumentV0, INITIAL_REVISION};
use dpp::identity::{KeyID, KeyType, Purpose, SecurityLevel};
use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use drive::dpp::platform_value::string_encoding::Encoding::Base58;
use drive::dpp::util::entropy_generator::DefaultEntropyGenerator;
use drive::dpp::version::PlatformVersion;
use getrandom::getrandom;
use simple_signer::signer::SimpleSigner;

use dpp::{
    dashcore::{self, key::Secp256k1},
};

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
    let data_contract_identifier: [u8; 32] = Identifier::from_string("2twstHkD3uYEogneYppHDCfnnfKxDk6YeJrKt3qNwtcW", Base58)
        .expect("Could not parse data contract identifier")
        .into();
    let document_identifier: [u8; 32] = Identifier::from_string("48dV7PmazUqPZjC7qZNpL2a9PiU9KzAHPsdkBUxrm7Yz", Base58)
        .expect("Could not parse data contract identifier")
        .into();
    let identity_identifier: [u8; 32] = Identifier::from_string("B7kcE1juMBWEWkuYRJhVdAE2e6RaevrGxRsa1DrLCpQH", Base58)
        .expect("Could not parse identity identifier")
        .into();


    let private_key = PrivateKey::from_wif("cTotPERUnsKgJgbddCKh1EqrBM2Esamu3V11rmn4jHSwhRtSb8y5")
        .expect("Could not parse pk");

    let public_key = private_key.public_key(&Secp256k1::new());
    let pubkey_hash = public_key.pubkey_hash();
    let address = pubkey_hash.to_hex();


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

    let document_properties = platform_value!(
     {
      "taskId": [
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
        ],
      "amountCredits": 20,
      "amountUSD": 500
    });

    let document_type_name = "Claim";

    let server_address: String = String::from("127.0.0.1");
    let core_port: u16 = 19998;
    let platform_port: u16 = 1443;
    let core_user: String = String::from("dashmate");
    let core_password: String = String::from("rTvfm81kiOxO");

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
    let document_contract_identifier = Identifier::from_bytes(&document_identifier).expect("parse data contract id");

    let contract: DataContract =
        DataContract::fetch(&sdk, data_contract_identifier).await.expect("fetch identity").expect("Data contract not found");

    // Now query for individual document
    let query = DocumentQuery::new(contract.clone(), &document_type_name)
        .expect("create SdkDocumentQuery")
        .with_document_id(&document_contract_identifier);

    let now = SystemTime::now();
    let now_seconds = now
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let entropy_generator = DefaultEntropyGenerator {};
    let entropy_buffer = entropy_generator.generate().unwrap();

    let document_id = Document::generate_document_id_v0(
        &data_contract_identifier,
        &identity_id,
        &document_type_name,
        entropy_buffer.as_slice(),
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


    let identity_public_key = identity.get_first_public_key_matching(
        Purpose::AUTHENTICATION,
        HashSet::from([SecurityLevel::HIGH]),
        HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
    )
        .expect("Could not match identity public key");


    let mut signer = SimpleSigner::default();

    let private_key_bytes = [];

    for (key_id, public_key) in identity.public_keys() {
        let identity_key_tuple = (identity_id, *key_id);

        signer
            .private_keys
            .insert(public_key.clone(), private_key_bytes.clone());
    }

    let data_contract_arc = Arc::new(contract.clone());


    let new_document = document.put_to_platform_and_wait_for_response(
        &sdk,
        new_document_type,
        entropy_generator.generate().unwrap(),
        identity_public_key.clone(),
        data_contract_arc,
        &signer,
    ).await.expect("There was a error pushing the document");

    println!("OK")
}
