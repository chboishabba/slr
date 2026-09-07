use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub user_agent: String,
    pub referer: Option<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub final_url: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

pub trait HttpTransport {
    type Error;
    fn get(&mut self, request: &HttpRequest) -> Result<HttpResponse, Self::Error>;
}

#[cfg(feature = "live-network")]
pub struct UreqTransport;

#[cfg(feature = "live-network")]
impl HttpTransport for UreqTransport {
    type Error = String;

    fn get(&mut self, request: &HttpRequest) -> Result<HttpResponse, Self::Error> {
        use std::io::Read;

        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(request.timeout_seconds))
            .build();
        let mut req = agent.get(&request.url).set("User-Agent", &request.user_agent);
        if let Some(referer) = &request.referer {
            req = req.set("Referer", referer);
        }
        let response = req.call().map_err(|err| err.to_string())?;
        let status_code = response.status();
        let final_url = response.get_url().to_string();
        let content_type = response.header("Content-Type").map(str::to_string);
        let mut body = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut body)
            .map_err(|err| err.to_string())?;
        Ok(HttpResponse {
            final_url,
            status_code,
            content_type,
            body,
        })
    }
}
