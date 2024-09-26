use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Context;
use dash_sdk::{mock::provider::GrpcContextProvider, platform, SdkBuilder};
use dash_sdk::dpp::data_contract::document_type::DocumentType;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::platform_value;
use dash_sdk::dpp::util::entropy_generator::EntropyGenerator;
use dash_sdk::platform::{DataContract, Document, DocumentQuery, Fetch, Identifier, Identity, Query};
use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::sdk::AddressList;
use dpp::dashcore::{PrivateKey};
use dpp::document::{DocumentV0, DocumentV0Getters, DocumentV0Setters, INITIAL_REVISION};
use drive::dpp::platform_value::string_encoding::Encoding::Base58;
use drive::dpp::version::PlatformVersion;
use getrandom::getrandom;
use simple_signer::signer::SimpleSigner;

use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};

use dotenv::dotenv;
use std::env;
use dash_sdk::platform::proto::get_documents_request::GetDocumentsRequestV0;
use dash_sdk::platform::proto::GetDocumentsRequest;
use dash_sdk::platform::transition::purchase_document::PurchaseDocument;
use dash_sdk::platform::transition::update_price_of_document::UpdatePriceOfDocument;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::fee::Credits;


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


  // ! IDENTIFIERS
  let data_contract_identifier: [u8; 32] = Identifier::from_string("CW12dnaPL3Mrb5MiJL4SgBhgimc4BBSagjL1dGtURcJp", Base58)
    .expect("Could not parse data contract identifier")
    .into();
  let identity_identifier: [u8; 32] = Identifier::from_string("9Upw4Yd8FmL6XvjTpAHguqWg227KkfRbmbhnfZFV7UuB", Base58)
    .expect("Could not parse identity identifier")
    .into();

  let document_identifier = Identifier::from_string("9NG46niBj1SQA6KD1M9CtZH6s2DGWmsfUdic4R9jySvR", Base58)
    .expect("Could not parse document identifier");


  let customer_identifier = Identifier::from_string("8eTDkBhpQjHeqgbVeriwLeZr1tCa6yBGw76SckvD1cwc", Base58)
    .expect("Could not parse customer identifier");


  // ! PRIVATE KEYS
  let private_key = PrivateKey::from_wif("cQ9xWG9f2gQjJ2uxqKDFFy7crSpziY4oADnPQfvGyQq3coKSo9XV")
    .expect("Could not parse pk");

  let customer_private_key = PrivateKey::from_wif("cV8gdL3T1syAMbg71EY7LuJAvdyVajE2XAzkdzHTw5AHmADt1pr6")
    .expect("Could not parse pk");


  // ! DATA CONTRACT DATA
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
    "required": ["name", "description", "url", "$createdAt", "$updatedAt"],
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
          "will_not_implement",
          "paid"
        ]
      }
    },
    "comment": {
      "position": 6,
      "type": "string",
      "description": "Change status comment",
      "maxLength": 1000
    },
    "required": ["title", "projectId", "$createdAt", "$updatedAt"],
    "additionalProperties": false
  },
  "Claim": {
    "type": "object",
    "transferable": 1,
    "tradeMode": 1,
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
      },
      "deliverable": {
        "position": 3,
        "type": "string",
        "description": "Claim deliverable",
        "maxLength": 255
      }
    },
    "required": [
      "$createdAt",
      "$updatedAt",
      "taskId",
      "amountCredits",
      "amountUSD",
      "deliverable"
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
      "amountUSD": 500,
       "deliverable": "https://github.com/LexxXell"
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

  // ! Create identity for seller
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
    revision: Option::from(INITIAL_REVISION as dpp::prelude::Revision),
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
  //
  //
  let mut signer = SimpleSigner::default();

  signer.add_key(identity_public_key.clone(), private_key.to_bytes().clone());

  let data_contract_arc = Arc::new(contract.clone());

  // ! ==================
  // !    PUT DOCUMENT
  // ! ==================

  // let new_document = document.put_to_platform_and_wait_for_response(
  //     &sdk,
  //     new_document_type,
  //     document_state_transition_entropy,
  //     identity_public_key.clone(),
  //     data_contract_arc,
  //     &signer,
  // ).await.expect("There was a error pushing the document");


  // print!("{:?}", new_document.to_string());


  // ! ==================
  // !    UPDATE PRICE
  // ! ==================

  let price: Credits = 200;


  let query = DocumentQuery::new_with_data_contract_id(&sdk, data_contract_identifier, document_type_name).await.expect("dq error");

  let test = query.with_document_id(&document_identifier);

  let mut document = Document::fetch(
    &sdk,
    test,
  ).await.expect("Cannot find document/data contract").unwrap();

  // document.set_revision(Option::from(document.revision().unwrap() + 1 as dpp::prelude::Revision));

  // let out = document.update_price_of_document_and_wait_for_response(
  //   price,
  //   &sdk,
  //   new_document_type,
  //   identity_public_key.clone(),
  //   data_contract_arc,
  //   &signer,
  // ).await.expect("Cannot set price");
  //
  // println!("{:?}", out);


  // ! ==================
  // !      PURCHASE
  // ! ==================

  // ! Create identity for customer
  let customer_identity_id = Identifier::from(customer_identifier);
  let customer_identity = Identity::fetch_by_identifier(&sdk, customer_identity_id).await.unwrap().expect("Identity not found");

  let customer_identity_public_key = customer_identity.get_public_key_by_id(1)
    .expect("Could not match identity public key");

  // ! SIGNER
  let mut customer_signer = SimpleSigner::default();

  customer_signer.add_key(customer_identity_public_key.clone(), customer_private_key.to_bytes().clone());

  let out = document.purchase_document_and_wait_for_response(
    price,
    &sdk,
    new_document_type,
    customer_identifier,
    customer_identity_public_key.clone(),
    data_contract_arc,
    &customer_signer,
  )
    .await
    .expect("Cannot purchase");

  // println!("{:?}", out);

  println!("OK")
}
