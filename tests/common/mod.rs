use mockito::{Mock, Server, ServerGuard};

#[allow(dead_code)]
pub fn mock_server() -> ServerGuard {
    Server::new()
}

#[allow(dead_code)]
pub fn mock_json(
    server: &mut ServerGuard,
    method: &str,
    path: &str,
    status: usize,
    body: serde_json::Value,
) -> Mock {
    server
        .mock(method, path)
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body(body.to_string())
        .create()
}

#[allow(dead_code)]
pub fn mock_text(
    server: &mut ServerGuard,
    method: &str,
    path: &str,
    status: usize,
    body: &str,
) -> Mock {
    server
        .mock(method, path)
        .with_status(status)
        .with_header("content-type", "text/plain")
        .with_body(body)
        .create()
}

#[allow(dead_code)]
pub fn mock_auth_401(server: &mut ServerGuard, path: &str) -> Mock {
    server
        .mock("GET", path)
        .with_status(401)
        .with_header("www-authenticate", "Basic realm=\"test\"")
        .with_body("unauthorized")
        .create()
}
