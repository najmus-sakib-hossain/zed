//! PubMed search engine implementation.
//!
//! Two-step XML API:
//! 1. esearch to get PMIDs
//! 2. efetch to get article details
//!
//!    Website: https://www.ncbi.nlm.nih.gov/pubmed
//!    Features: Paging

use async_trait::async_trait;
use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::Result,
    query::SearchQuery,
    result::SearchResult,
};
use regex::Regex;
use reqwest::Client;
use tracing::info;
use smallvec::smallvec;

pub struct Pubmed {
    metadata: EngineMetadata,
    client: Client,
}

impl Pubmed {
    pub fn new(client: Client) -> Self {
        Self {
            metadata: EngineMetadata {
                name: "pubmed".to_string().into(),
                display_name: "PubMed".to_string().into(),
                homepage: "https://www.ncbi.nlm.nih.gov/pubmed".to_string().into(),
                categories: smallvec![SearchCategory::Science],
                enabled: true,
                timeout_ms: 8000,
                weight: 1.0,
            },
            client,
        }
    }

    /// Extract all text content between matching XML tags.
    fn extract_tag_values(xml: &str, tag: &str) -> Vec<String> {
        let pattern = format!(r"<{0}[^>]*>([\s\S]*?)</{0}>", regex::escape(tag));
        let re = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        re.captures_iter(xml)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
            .collect()
    }

    /// Extract the first text content between matching XML tags.
    fn extract_first_tag(xml: &str, tag: &str) -> Option<String> {
        Self::extract_tag_values(xml, tag).into_iter().next()
    }

    /// Strip XML tags from a string.
    fn strip_xml_tags(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut in_tag = false;
        for ch in s.chars() {
            if ch == '<' {
                in_tag = true;
            } else if ch == '>' {
                in_tag = false;
            } else if !in_tag {
                result.push(ch);
            }
        }
        result
    }
}

#[async_trait]
impl SearchEngine for Pubmed {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let encoded = urlencoding::encode(&query.query);
        let retmax = 10u32;
        let retstart = (query.page.max(1) - 1) * retmax;

        // Step 1: Search for PMIDs
        let search_url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term={}&retstart={}&retmax={}",
            encoded, retstart, retmax
        );

        let search_resp = match self
            .client
            .get(&search_url)
            .timeout(std::time::Duration::from_secs(3))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let search_body = match search_resp.text().await {
            Ok(b) => b,
            Err(_) => return Ok(Vec::new()),
        };

        // Extract PMIDs from <IdList><Id>...</Id></IdList>
        let id_list_xml = match Self::extract_first_tag(&search_body, "IdList") {
            Some(xml) => xml,
            None => return Ok(Vec::new()),
        };

        let pmids = Self::extract_tag_values(&id_list_xml, "Id");
        if pmids.is_empty() {
            return Ok(Vec::new());
        }

        let ids_param = pmids.join(",");

        // Step 2: Fetch article details
        let fetch_url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi?db=pubmed&retmode=xml&id={}",
            ids_param
        );

        let fetch_resp = match self
            .client
            .get(&fetch_url)
            .timeout(std::time::Duration::from_secs(4))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(Vec::new()),
        };

        let fetch_body = match fetch_resp.text().await {
            Ok(b) => b,
            Err(_) => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        // Split by <PubmedArticle> to process each article
        for (i, article_chunk) in fetch_body.split("<PubmedArticle>").skip(1).enumerate() {
            let article = match article_chunk.split("</PubmedArticle>").next() {
                Some(a) => a,
                None => continue,
            };

            // Extract PMID
            let pmid = match Self::extract_first_tag(article, "PMID") {
                Some(id) => Self::strip_xml_tags(&id),
                None => continue,
            };

            // Extract title
            let title = match Self::extract_first_tag(article, "ArticleTitle") {
                Some(t) => Self::strip_xml_tags(&t),
                None => continue,
            };

            if title.is_empty() {
                continue;
            }

            let result_url = format!("https://pubmed.ncbi.nlm.nih.gov/{}/", pmid);

            // Extract abstract
            let abstract_text = Self::extract_first_tag(article, "AbstractText")
                .map(|a| Self::strip_xml_tags(&a))
                .unwrap_or_default();

            // Extract authors
            let mut authors = Vec::new();
            for author_chunk in article.split("<Author ").skip(1) {
                let author_block = match author_chunk.split("</Author>").next() {
                    Some(a) => a,
                    None => continue,
                };
                let forename = Self::extract_first_tag(author_block, "ForeName")
                    .map(|s| Self::strip_xml_tags(&s))
                    .unwrap_or_default();
                let lastname = Self::extract_first_tag(author_block, "LastName")
                    .map(|s| Self::strip_xml_tags(&s))
                    .unwrap_or_default();
                if !lastname.is_empty() {
                    if forename.is_empty() {
                        authors.push(lastname);
                    } else {
                        authors.push(format!("{} {}", forename, lastname));
                    }
                }
            }

            let content = if authors.is_empty() {
                abstract_text.chars().take(300).collect()
            } else {
                let author_str = authors.join(", ");
                format!(
                    "[{}] {}",
                    author_str,
                    abstract_text.chars().take(250).collect::<String>()
                )
            };

            let mut r = SearchResult::new(&title, &result_url, &content, "pubmed");
            r.engine_rank = i as u32;
            r.category = SearchCategory::Science.to_string();
            results.push(r);
        }

        info!(
            engine = "pubmed",
            count = results.len(),
            "Search complete"
        );
        Ok(results)
    }
}
