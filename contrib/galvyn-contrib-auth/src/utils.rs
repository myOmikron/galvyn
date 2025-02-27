use openidconnect::reqwest::Error as OidcError;

/// Function used to send the actual openid connect requests
pub async fn async_http_client(
    request: oauth2::HttpRequest,
) -> Result<oauth2::HttpResponse, OidcError<reqwest::Error>> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // TODO
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(OidcError::Reqwest)?;

    let mut request_builder = client
        .request(request.method, request.url.as_str())
        .body(request.body);
    for (name, value) in &request.headers {
        request_builder = request_builder.header(name.as_str(), value.as_bytes());
    }
    let response = request_builder.send().await.map_err(OidcError::Reqwest)?;

    let status_code = response.status();
    let headers = response.headers().to_owned();
    let chunks = response.bytes().await.map_err(OidcError::Reqwest)?;
    Ok(oauth2::HttpResponse {
        status_code,
        headers,
        body: chunks.to_vec(),
    })
}
