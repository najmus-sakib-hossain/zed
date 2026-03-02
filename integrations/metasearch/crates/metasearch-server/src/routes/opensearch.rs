//! OpenSearch descriptor endpoint.
//!
//! GET /opensearch.xml
//!
//! Returns an OpenSearch XML descriptor that allows browsers (Firefox, Chrome)
//! to add Metasearch as a search engine in their search bar.

use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
};

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/opensearch.xml", get(opensearch_handler))
}

async fn opensearch_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let base_url = &state.settings.server.base_url;
    
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/"
                       xmlns:moz="http://www.mozilla.org/2006/browser/search/">
  <ShortName>Metasearch</ShortName>
  <Description>Privacy-respecting metasearch engine aggregating 200+ sources</Description>
  <InputEncoding>UTF-8</InputEncoding>
  <Image width="16" height="16" type="image/svg+xml">data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%230070f3' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'><circle cx='11' cy='11' r='8'/><path d='m21 21-4.35-4.35'/></svg></Image>
  <Url type="text/html" method="get" template="{base_url}/search?q={{searchTerms}}&amp;category={{category?}}"/>
  <Url type="application/x-suggestions+json" method="get" template="{base_url}/autocomplete?q={{searchTerms}}"/>
  <moz:SearchForm>{base_url}/</moz:SearchForm>
</OpenSearchDescription>"#
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/opensearchdescription+xml")],
        xml,
    )
}
