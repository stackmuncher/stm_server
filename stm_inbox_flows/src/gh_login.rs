use hyper::{Client, Request};
use hyper_rustls::HttpsConnector;
use regex::Regex;
use ring::signature;
use serde::Deserialize;
use serde_json::Value;
use stm_shared::log_http_body;
use tracing::{debug, error, info};

/// A "well-known" string used as the content to be signed for GH verification. The signature is uploaded to a Gist.
const GH_VERIFICATION_STRING_TO_SIGN: &str = "stackmuncher";

/// A stripped-down representation of GH GetGist API response: Owner details.
#[derive(Deserialize)]
pub(crate) struct GistOwner {
    /// GitHub login of the user, e.g. `rimutaka`.
    login: Option<String>,
}

/// A rough top-level representation of GH GetGist API response.
#[derive(Deserialize)]
pub(crate) struct RawGist {
    /// The file name is used as the property name, so it is easier to just get Value and then manually dig into it.
    /// We only need the contents.
    /// ```json
    /// "files": {
    ///   "stm.txt": {
    ///     "content": "MDQ6R2lzdGZiOGZjMGY4N2VlNzgyMzFmMDY0MTMxMDIyYzgxNTRh"
    ///   }
    /// }
    /// ```
    pub files: Option<Value>,
    pub owner: Option<GistOwner>,
}

/// Returns GH login after validating the Gist contents, if any for the given Gist ID. Can be tested with this shell command:
/// ```shell
/// curl \
///  -H "Accept: application/vnd.github.v3+json" \
///  https://api.github.com/gists/GIST_ID
/// ```
pub(crate) async fn get_validated_gist(
    gist_id: &Option<String>,
    pub_key: &String,
    gh_login_invalidation_regex: &Regex,
) -> Option<String> {
    // remove GH login info if gist_is is empty - that's because the user reset it to empty and wants GH unlinked
    let gist_id = match gist_id {
        Some(v) => v,
        None => {
            info!("Removing gh_login for {}", pub_key);
            return None;
        }
    };

    info!("Getting GitHub validation from Gist #{}", gist_id);

    let uri = ["https://api.github.com/gists/", &gist_id].concat();

    // prepare the HTTP request to GitHub API
    let req = Request::builder()
        .uri(uri.clone())
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "StackMuncher App")
        .method("GET")
        .body(hyper::Body::empty())
        .expect("Cannot create Gist API request");
    debug!("Http rq: {:?}", req);

    // send it out, but it may fail for any number of reasons and we still have to carry on
    let res = match Client::builder()
        .build::<_, hyper::Body>(HttpsConnector::with_native_roots())
        .request(req)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("GitHub API request to {} failed with {}", uri, e);
            return None;
        }
    };

    let status = res.status();
    info!("GH API response status: {}", status);

    // Concatenate the body stream into a single buffer...
    let buf = hyper::body::to_bytes(res)
        .await
        .expect("Cannot convert GH API response body to bytes. It's a bug.");

    // there should be at least some data returned
    if buf.is_empty() {
        error!("Empty GH API response with status {}", status);
        return None;
    }

    // any status other than 200 is an error
    if !status.is_success() {
        error!("Status {}", status);
        log_http_body(&buf);
        return None;
    }

    // all responses should be JSON. If it's not JSON it's an error.
    let gist = match serde_json::from_slice::<RawGist>(&buf) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to convert GH API response to JSON with {}", e);
            log_http_body(&buf);
            return None;
        }
    };
    info!("GH API response arrived");

    // check that all the data we need is in there
    let github_login = match gist.owner {
        Some(v) => match v.login {
            Some(v) => v,
            None => {
                error!("Invalid GH API response: missing `owner/login` JSON property");
                log_http_body(&buf);
                return None;
            }
        },
        None => {
            error!("Invalid GH API response: missing `owner` JSON property");
            log_http_body(&buf);
            return None;
        }
    };
    info!("Gist owner: {}", github_login);

    // validate if the GH Login has any chars outside of the expected range
    if !validate_gh_login_format(&github_login, gh_login_invalidation_regex) {
        return None;
    }

    // are there any GIST contents at all?
    // expecting something like
    // "files": {"stm.txt": { "content": "MDQ6R2lzdGZiOGZjMGY4N2VlNzgyMzFmMDY0MTMxMDIyYzgxNTRh" } }
    let gist_contents = match gist.files {
        None => {
            error!("Invalid GH API response: missing `file` JSON property");
            log_http_body(&buf);
            return None;
        }
        Some(v) => v,
    };

    // there should be just one property inside "files", but we don't know its name because it is the file name
    // which can be anything
    // the insanely deep check is to make the code more readable - not very efficient, but it's OK, only run once in a while
    if !gist_contents.is_object()
        || gist_contents.as_object().is_none()
        || gist_contents.as_object().unwrap().len() != 1
        || gist_contents.as_object().unwrap().iter().next().is_none()
        || !gist_contents.as_object().unwrap().iter().next().unwrap().1.is_object()
        || gist_contents
            .as_object()
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .1
            .get("content")
            .is_none()
        || !gist_contents
            .as_object()
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .1
            .get("content")
            .unwrap()
            .is_string()
    {
        error!("Invalid GH API response: invalid `file` JSON property");
        log_http_body(&buf);
        return None;
    }

    // this is the actual file, so the property name is "stm.txt" in our example and we can try getting "content"
    let gist_contents = gist_contents
        .as_object()
        .unwrap()
        .iter()
        .next()
        .unwrap()
        .1
        .get("content")
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned();

    // remove possible wrappers and white space around it
    let gist_contents = gist_contents
        .replace("\"", "")
        .replace("'", "")
        .replace("`", "")
        .trim()
        .to_string();

    if gist_contents.len() > 300 {
        error!("Gist contents is too long: {}", gist_contents.len());
        return None;
    }

    // convert the signature from base58 into bytes
    let signature = match bs58::decode(gist_contents.clone()).into_vec() {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to decode the contents of the Gist from based58 due to: {}", e);
            return None;
        }
    };

    // convert pub_key from base58 into bytes
    let pub_key = match bs58::decode(pub_key.clone()).into_vec() {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to decode pub_key(owner_id) from based58 due to: {}", e);
            return None;
        }
    };

    // check if the signature in the gist is valid
    let pub_key = signature::UnparsedPublicKey::new(&signature::ED25519, pub_key);
    match pub_key.verify(GH_VERIFICATION_STRING_TO_SIGN.as_bytes(), &signature) {
        Ok(_) => {
            info!("Signature OK");
        }
        Err(_) => {
            error!("Invalid signature in Gist: {}", gist_contents);
            return None;
        }
    };

    Some(github_login)
}

/// Logs and error and returns false if `gh_login` is empty or has any characters outside of the allowed range.
/// Otherwise returns true.
pub(crate) fn validate_gh_login_format(gh_login: &String, gh_login_invalidation_regex: &Regex) -> bool {
    // check if the login is save, even if we got it from GH
    if gh_login.is_empty() || gh_login.len() > 150 || gh_login_invalidation_regex.is_match(gh_login) {
        error!("Invalid GitHub Login format: {}", gh_login);
        false
    } else {
        true
    }
}
