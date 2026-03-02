use url::Url;

use crate::{
    engines::{EngineResponse, RequestResponse, CLIENT},
    parse::{parse_html_response_with_opts, ParseOpts},
};

pub async fn request(query: &str) -> RequestResponse {
    CLIENT
        .get(Url::parse_with_params("https://search.brave.com/search", &[("q", query)]).unwrap())
        .into()
}

pub fn parse_response(body: &str) -> eyre::Result<EngineResponse> {
    parse_html_response_with_opts(
        body,
        ParseOpts::new()
            .result("#results > .snippet[data-pos]:not(.standalone)")
            .title(".title")
            .href("a")
            .description(".generic-snippet, .video-snippet > .snippet-description"),
    )
}
