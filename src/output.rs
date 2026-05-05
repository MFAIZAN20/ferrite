pub mod printer;
pub mod theme;

use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::cli::CliArgs;
use crate::config::Config;
use crate::response::{RequestTrace, ResponseData};
#[allow(unused_imports)]
pub use printer::{
    build_print_opts, parse_print_flag, print_request, print_response, PrettyMode, PrintOpts,
};

/// Backward-compatible high-level renderer now backed by the new printer system.
pub fn render_exchange_from_cli(
    request: &RequestTrace,
    response: &ResponseData,
    cli: &CliArgs,
    config: &Config,
) -> Result<()> {
    let opts = build_print_opts(cli, config);
    let req_headers = vec_headers_to_map(&request.headers);
    let req_body = request
        .body_preview
        .as_deref()
        .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok());
    print_request(
        &request.method,
        &request.url,
        &req_headers,
        req_body.as_ref(),
        &opts,
    );

    let res_headers = vec_headers_to_map(&response.headers);
    print_response(
        response.status_code,
        &response.reason,
        &res_headers,
        &response.body,
        response.content_type.as_deref().unwrap_or(""),
        &opts,
    );
    Ok(())
}

fn vec_headers_to_map(input: &[(String, String)]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (k, v) in input {
        let Ok(name) = HeaderName::from_bytes(k.as_bytes()) else {
            continue;
        };
        let Ok(value) = HeaderValue::from_str(v) else {
            continue;
        };
        headers.append(name, value);
    }
    headers
}
