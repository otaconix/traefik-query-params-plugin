use traefik_wasm_api as traefik;

#[unsafe(export_name = "handle_request")]
fn http_request() -> i64 {
    let request_uri = traefik::get_request_uri();
    let mut url = url::Url::parse(&format!("http://example.invalid{request_uri}")).unwrap();
    url.query_pairs_mut()
        .append_pair("server", "https://matrix.zwanenburg.info");
    traefik::set_request_uri(&format!(
        "{}{}",
        url.path(),
        url.query()
            .map(|query| format!("?{query}"))
            .unwrap_or("".to_string())
    ));

    1
}

#[unsafe(export_name = "handle_response")]
fn http_response(_req_ctx: i32, _is_error: i32) {}

fn main() {}
