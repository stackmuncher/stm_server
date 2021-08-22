use crate::config::Config;
use crate::s3;
use base64::decode;
use lambda_runtime::{Context, Error};
use ring::signature;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayResponse {
    is_base64_encoded: bool,
    status_code: u32,
    headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ApiGatewayRequestHeaders {
    /// The user public key, which may or may not be known to us at the time of submission.
    /// A base58 encoded string, e.g. "EFY9NXEytYgBgGsyAeGfXzkBEBQzC9NXFyj47EPdmVLB"
    stackmuncher_key: Option<String>,
    /// The signature for the content sent in the body, base58 encoded, e.g.
    /// "3phLLQyiquyX4xge3CXYGCfb1KdrXQ8cTgBbvE8obwCkcm7vPdLsKT6JtNCdF9qeyjcgF2b4kTRXEsoMTHcQr43n"
    stackmuncher_sig: Option<String>,
    /// The IP address of the user. Apparently it is preferred over sourceIp field.
    /// See https://docs.aws.amazon.com/elasticloadbalancing/latest/classic/x-forwarded-headers.html
    #[serde(rename = "x-forwarded-for")]
    x_forwarded_for: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayRequest {
    headers: ApiGatewayRequestHeaders,
    is_base64_encoded: bool,
    body: Option<String>,
}

/// A generic error message sent to the user when the request cannot be processed for a reason the user can't do much about.
const ERROR_500_MSG: &str = "stackmuncher.com failed to process the report. If the error persists, can you log an issue at https://github.com/stackmuncher/stm_inbox/issues?";

pub(crate) async fn my_handler(event: Value, ctx: Context, config: &Config) -> Result<Value, Error> {
    // these 2 lines are for debugging only to see the raw APIGW request
    debug!("Event: {}", event);
    debug!("Context: {:?}", ctx);

    // parse the request
    let api_request = match serde_json::from_value::<ApiGatewayRequest>(event.clone()) {
        Err(e) => {
            error!("Failed to deser APIGW request due to {}. Request: {}", e, event);
            return gw_response(Some(ERROR_500_MSG.to_owned()), 500);
        }
        Ok(v) => v,
    };

    info!("Report from IP: {:?}", api_request.headers.x_forwarded_for);

    // these 2 headers are required no matter what
    if api_request.headers.stackmuncher_key.is_none() || api_request.headers.stackmuncher_sig.is_none() {
        error!(
            "Missing a header. Key: {:?}, Sig: {:?}",
            api_request.headers.stackmuncher_key, api_request.headers.stackmuncher_sig
        );
        return gw_response(
            Some("stackmuncher.com failed to process the report: missing required HTTP headers. If you have not modified the source code it's a bug at stackmuncher.com end.".to_owned()),
            500,
        );
    }

    // get the body contents and decode it if needed
    let body = match api_request.body {
        Some(v) => v,
        None => {
            error!("Empty body");
            return gw_response(
            Some("stackmuncher.com: no report found in the request. It's a bug in the app. Can you log an issue at https://github.com/stackmuncher/stm_inbox/issues?".to_owned()),
            500,
        );
        }
    };
    let body = if api_request.is_base64_encoded {
        match decode(body) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to decode the body due to: {}", e);
                return gw_response(Some(ERROR_500_MSG.to_owned()), 500);
            }
        }
    } else {
        body.as_bytes().into()
    };

    info!("Body len: {}", body.len());
    debug!("Body: {}", String::from_utf8_lossy(&body));

    // convert the public key from base58 into bytes
    let pub_key_bs58 = api_request
        .headers
        .stackmuncher_key
        .expect("Cannot unwrap stackmuncher_key. It's a bug.");

    info!("Report for pub key: {}", pub_key_bs58);

    if !validate_owner_id(&pub_key_bs58) {
        error!("Invalid pub key length: {}", pub_key_bs58.len());
        return gw_response(Some("Invalid public key length. Expecting 32 bytes as base58.".to_owned()), 403);
    }

    let pub_key = match bs58::decode(pub_key_bs58.clone()).into_vec() {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to decode the stackmuncher_key from based58 due to: {}", e);
            return gw_response(Some("Failed to decode public key from based58".to_owned()), 403);
        }
    };

    // convert the signature from base58 into bytes
    let signature = match bs58::decode(
        api_request
            .headers
            .stackmuncher_sig
            .expect("Cannot unwrap stackmuncher_sig. It's a bug."),
    )
    .into_vec()
    {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to decode the stackmuncher_key from based58 due to: {}", e);
            return gw_response(Some(ERROR_500_MSG.to_owned()), 500);
        }
    };

    // validate the signature
    let pub_key = signature::UnparsedPublicKey::new(&signature::ED25519, pub_key);
    match pub_key.verify(&body, &signature) {
        Ok(_) => {
            info!("Signature OK");
        }
        Err(e) => {
            error!("Invalid signature: {}", e);
            return gw_response(Some("Invalid StackMuncher signature. If the error persists, can you log an issue at https://github.com/stackmuncher/stm_inbox/issues?".to_owned()), 500);
        }
    };

    s3::upload_to_s3(&config, body, pub_key_bs58).await;

    // render the prepared data as HTML
    info!("Report stored");

    // Submission accepted - return 200 with no body
    gw_response(None, 200)
}

/// Prepares the response with the status and text or json body. May fail and return an error.
fn gw_response(body: Option<String>, status_code: u32) -> Result<Value, Error> {
    let mut headers: HashMap<String, String> = HashMap::new();
    headers.insert("Content-Type".to_owned(), "text/text".to_owned());
    headers.insert("Cache-Control".to_owned(), "no-store".to_owned());

    let resp = ApiGatewayResponse {
        is_base64_encoded: false,
        status_code,
        headers,
        body,
    };

    Ok(serde_json::to_value(resp).expect("Failed to serialize response"))
}

/// Returns TRUE if the owner_id decodes from base58 into exactly 256 bytes.
/// Logs a warning and returns FALSE otherwise.
/// TODO: this should be a shared utility function!!!
fn validate_owner_id(owner_id: &str) -> bool {
    match bs58::decode(owner_id).into_vec() {
        Err(e) => {
            warn!("Invalid owner_id: {}. Cannot decode from bs58: {}", owner_id, e);
            false
        }
        Ok(v) => {
            if v.len() == 32 {
                true
            } else {
                warn!("Invalid owner_id: {}. Decoded to {} bytes", owner_id, v.len());
                false
            }
        }
    }
}
