use crate::api_gw_request::ApiGatewayRequest;
use crate::config::Config;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tracing::error;

/// A JWT struct with fields of interest. Supports LinkedIn.
/// Example: `eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6IlVLa2JMNU91M3lBck5XSXhDZzFVciJ9.eyJnaXZlbl9uYW1lIjoiTWF4IiwiZmFtaWx5X25hbWUiOiJWb3Nrb2IiLCJuaWNrbmFtZSI6Im1heCIsIm5hbWUiOiJNYXggVm9za29iIiwicGljdHVyZSI6Imh0dHBzOi8vbWVkaWEtZXhwMS5saWNkbi5jb20vZG1zL2ltYWdlL0M1MTAzQVFFNDg4YUcyblNVSXcvcHJvZmlsZS1kaXNwbGF5cGhvdG8tc2hyaW5rXzgwMF84MDAvMC8xNTIxMzExMjM2MzgyP2U9MTY0OTg5NDQwMCZ2PWJldGEmdD10M3R0b1dGS21rZmZ1V0lNQUx5WHBZUFl0eVVjZkRma0paSlctNmxXUGhNIiwidXBkYXRlZF9hdCI6IjIwMjItMDItMDZUMjI6MDQ6MzIuMjkyWiIsImVtYWlsIjoibWF4QG9uZWJyby5tZSIsImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJpc3MiOiJodHRwczovL3N0YWNrbXVuY2hlci51cy5hdXRoMC5jb20vIiwic3ViIjoibGlua2VkaW58MlE5Wk5tQmtVYyIsImF1ZCI6IlpmMlM0Q2tIUmU5TTdsNzRKMUFqRGdheFl1b291akgwIiwiaWF0IjoxNjQ0MzcwNTcwLCJleHAiOjE2NDQ0MDY1NzAsIm5vbmNlIjoiZVZkVmExODBRbll6UnpGaVRYUjBlRk0yWHpsNVdTNDNVRTV0YkRBMmJ6Tm1PVlJCUVhvMFkwOXNRZz09In0.MduEiyBnFg97ns4RqfP_VXg00RXBAYPdjUzXntygS7yZI8OvN6zdush13tfMumcw0nod0OSVTtAHMKMc7Js0o1sjtSe0Vt8MuRcZkTjiaXUEwGc-URMpb1UWJIGI98NgjunKzxeEiJnjK85bZwSFYdS2DhcvYe9avJ6uzQSefkUD-fEiHCC7ZDx4CIk3yOACFRKIvmshabqCobOJnIc3oJlhNal3XTY-IlvQylGWbBTZXidqdJrPTanuAEFcnpgyVaXRi-s_ykV5hjcuKI28k2Y2OTg4F_2IpJial77WRRLmHLpdedP_8qid8YqEeAEIDq3FugEn7r9zpFuNqH_CXQ`
/// Decode it with https://jwt.io
#[derive(Deserialize, Debug)]
pub(crate) struct JwtClaims {
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    // pub nickname: Option<String>,
    pub name: Option<String>,
    // pub picture: Option<String>,
    // pub updated_at: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    // pub iss: String,
    // pub sub: String,
    // pub aud: String,
    // pub iat: i64,
    // pub exp: i64,
}

/// Returns `true` if the request contains a valid JWT. Otherwise `false`.
/// Logs errors and tries not to panic.
pub(crate) fn validate_jwt(req: &ApiGatewayRequest, config: &Config) -> Option<JwtClaims> {
    // extract the token from the request
    // "authorization":"Bearer eyJhbGciOiJSUzI..."
    let jwt_header = match req.headers.get("authorization") {
        Some(v) => v.trim().trim_start_matches("Bearer ").trim_start().to_string(),
        None => {
            error!("Missing authorization header");
            return None;
        }
    };

    // a key for validating the token
    // the values come from https://stackmuncher.us.auth0.com/.well-known/jwks.json and are public
    let decoding_key = match DecodingKey::from_rsa_components(&config.jwk_n, &config.jwk_e) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid decoding key in the config: {} / {} / {}", config.jwk_n, config.jwk_e, e);
            return None;
        }
    };

    // validate the signature - other fields like issuer or audience are not validated
    // it is OK as long as there is just one application for single audience
    let jwt = match decode::<JwtClaims>(&jwt_header, &decoding_key, &Validation::new(Algorithm::RS256)) {
        Ok(v) => v,
        Err(e) => {
            error!("Invalid token: {} / {}", jwt_header, e);
            return None;
        }
    };

    // check that all the claims we need is there - must have an email
    if jwt.claims.email.is_none() || jwt.claims.email.as_ref().unwrap().is_empty() {
        error!("Empty email for token: {}", jwt_header);
        return None;
    }

    // the email must be verified
    if jwt.claims.email_verified.is_none() || !jwt.claims.email_verified.as_ref().unwrap() {
        error!("Unverified email for token: {}", jwt_header);
        return None;
    }

    // must have at least some sort of a name
    if (jwt.claims.given_name.is_none() || jwt.claims.given_name.as_ref().unwrap().is_empty())
        && (jwt.claims.family_name.is_none() || jwt.claims.given_name.as_ref().unwrap().is_empty())
        && (jwt.claims.name.is_none() || jwt.claims.name.as_ref().unwrap().is_empty())
    {
        error!("Empty name for token: {}", jwt_header);
        return None;
    }

    Some(jwt.claims)
}
