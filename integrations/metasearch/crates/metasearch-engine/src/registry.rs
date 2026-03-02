//! Engine registry — manages all available search engines.

use std::sync::Arc;

use dashmap::DashMap;
use metasearch_core::category::SearchCategory;
use metasearch_core::engine::SearchEngine;
use reqwest::Client;

use crate::{
    acfun::AcFun, adobe_stock::AdobeStock, ads::Ads, ahmia::Ahmia, alpinelinux::AlpineLinux,
    annas_archive::AnnasArchive, ansa::Ansa, apkmirror::ApkMirror, apple_app_store::AppleAppStore,
    apple_maps::AppleMaps, archlinux::ArchLinux, artic::Artic, artstation::ArtStation,
    arxiv::Arxiv, ask::Ask, baidu::Baidu, bandcamp::Bandcamp, base_search::BaseSearch,
    bilibili::Bilibili, bing::Bing, bing_images::BingImages, bing_news::BingNews,
    bing_videos::BingVideos, bitchute::BitChute, bpb::Bpb, brave::Brave, braveapi::BraveApi,
    bt4g::Bt4g, btdigg::Btdigg, cachy_os::CachyOs, ccc_media::CccMedia, chefkoch::Chefkoch,
    chinaso::Chinaso, core_engine::CoreEngine, crates_io::CratesIo, crossref::Crossref,
    currency_convert::CurrencyConvert, dailymotion::Dailymotion, deepl::DeepL, deezer::Deezer,
    destatis::Destatis, deviantart::DeviantArt, devicons::Devicons, dictzone::DictZone,
    digbt::Digbt, discourse::Discourse, docker_hub::DockerHub, doku::Doku, duckduckgo::DuckDuckGo,
    duckduckgo_definitions::DuckDuckGoDefinitions, duckduckgo_weather::DuckDuckGoWeather,
    duden::Duden, ebay::Ebay, elasticsearch_engine::ElasticsearchEngine, emojipedia::Emojipedia,
    fdroid::Fdroid, findthatmeme::FindThatMeme, flickr::Flickr, flickr_noapi::FlickrNoapi,
    freesound::Freesound, frinkiac::Frinkiac, fyyd::Fyyd, geizhals::Geizhals, genius::Genius,
    gitea::Gitea, github_engine::GitHub, gitlab::GitLab, goodreads::Goodreads, google::Google,
    google_images::GoogleImages, google_news::GoogleNews, google_play::GooglePlay,
    google_scholar::GoogleScholar, google_videos::GoogleVideos, grokipedia::Grokipedia,
    hackernews::HackerNews, hex::Hex, huggingface::HuggingFace, il_post::IlPost, imdb::Imdb,
    imgur::Imgur, ina::Ina, invidious::Invidious, ipernity::Ipernity, iqiyi::Iqiyi, jisho::Jisho,
    kickass::Kickass, leet_x::LeetX, lemmy::Lemmy, lib_rs::LibRs, libretranslate::LibreTranslate,
    lingva::Lingva, livespace::LiveSpace, loc::Loc, lucide::Lucide, marginalia::Marginalia,
    mastodon::Mastodon, material_icons::MaterialIcons, mediathekviewweb::MediathekViewWeb,
    mediawiki_engine::MediaWikiEngine, meilisearch_engine::MeilisearchEngine, metacpan::MetaCpan,
    mixcloud::Mixcloud, mojeek::Mojeek, moviepilot::Moviepilot, mwmbl::Mwmbl, naver::Naver,
    nine_gag::NineGag, npm::Npm, nyaa::Nyaa, odysee::Odysee, openalex::OpenAlex,
    openlibrary::OpenLibrary, openstreetmap::OpenStreetMap, openverse::Openverse,
    peertube::PeerTube, photon::Photon, pinterest::Pinterest, piped::Piped, piratebay::PirateBay,
    pkg_go_dev::PkgGoDev, podcastindex::PodcastIndex, pypi::PyPI, quark::Quark, qwant::Qwant,
    radio_browser::RadioBrowser, recoll_engine::RecollEngine, reddit::Reddit,
    rightdao::RightDao,
    rottentomatoes::RottenTomatoes, rumble::Rumble, searchcode::Searchcode,
    semantic_scholar::SemanticScholar, sepiasearch::SepiaSearch, sogou::Sogou,
    solidtorrents::SolidTorrents, soundcloud::SoundCloud, sourcehut::Sourcehut, spotify::Spotify,
    springer::Springer, stackexchange::StackExchange, stract::Stract, svgrepo::SvgRepo,
    tagesschau::Tagesschau, three_sixty_search_videos::ThreeSixtySearchVideos, tineye::TinEye,
    tokyotoshokan::TokyoToshokan, tootfinder::Tootfinder, unsplash::Unsplash, vimeo::Vimeo,
    voidlinux::VoidLinux, wallhaven::Wallhaven, wikicommons::WikiCommons, wikipedia::Wikipedia,
    wordnik::Wordnik, yahoo::Yahoo, yandex::Yandex, yep::Yep, youtube::YouTube,
};
use crate::{
    microsoft_learn::MicrosoftLearn, mrs::Mrs, nvd::Nvd, openclipart::Openclipart,
    pdbe::Pdbe, repology::Repology, reuters::Reuters, scanr_structures::ScanrStructures,
    searchcode_code::SearchcodeCode, selfhst::Selfhst, senscritique::SensCritique,
    sogou_images::SogouImages, sogou_videos::SogouVideos, sogou_wechat::SogouWechat,
    steam::SteamStore, uxwing::Uxwing, www1x::Www1x, yahoo_news::YahooNews,
};
use crate::{
    astrophysics_data_system::AstrophysicsDataSystem, presearch::Presearch,
    wolframalpha_api::WolframAlphaApi, wolframalpha_noapi::WolframAlphaNoapi,
    youtube_api::YoutubeApi, youtube_noapi::YoutubeNoapi,
};
use crate::{
    github_code::GithubCode, niconico::Niconico, pixabay::Pixabay, pubmed::Pubmed,
    yandex_music::YandexMusic, zlibrary::Zlibrary,
};
use crate::{
    mozhi::Mozhi, open_meteo::OpenMeteo, seznam::Seznam, startpage::Startpage,
    translated::Translated, wttr::Wttr,
};
use crate::{
    duckduckgo_extra::DuckDuckGoExtra, pexels::Pexels, pixiv::Pixiv,
    three_sixty_search::ThreeSixtySearch, torznab::Torznab, yacy::Yacy,
};
use crate::{
    opensemantic::OpenSemantic, public_domain_image_archive::PublicDomainImageArchive,
    searx_engine::SearxEngine, solr::Solr, tubearchivist::TubeArchivist, wikidata::Wikidata,
};
use crate::{azure::Azure, cloudflareai::CloudflareAi, ollama::Ollama};

/// Central registry of all search engines.
pub struct EngineRegistry {
    engines: DashMap<String, Arc<dyn SearchEngine>>,
}

impl EngineRegistry {
    pub fn new() -> Self {
        Self {
            engines: DashMap::new(),
        }
    }

    /// Create a registry pre-loaded with all built-in engines (209 total).
    pub fn with_defaults(client: Client) -> Self {
        let mut registry = Self::new();
        // Note: reqwest::Client is already Arc-backed internally — no need to wrap

        // ── Original engines ──────────────────────────────
        registry.register(Arc::new(Google::new(client.clone())));
        registry.register(Arc::new(DuckDuckGo::new(client.clone())));
        registry.register(Arc::new(Brave::new(client.clone(), None)));
        registry.register(Arc::new(Wikipedia::new(client.clone())));

        // ── Batch 1: SearXNG translations ─────────────────
        registry.register(Arc::new(Bing::new(client.clone())));
        registry.register(Arc::new(Arxiv::new(client.clone())));
        registry.register(Arc::new(Ask::new(client.clone())));
        registry.register(Arc::new(Bandcamp::new(client.clone())));
        registry.register(Arc::new(Baidu::new(client.clone())));
        registry.register(Arc::new(NineGag::new(client.clone())));
        registry.register(Arc::new(AppleAppStore::new(client.clone())));
        registry.register(Arc::new(Bilibili::new(client.clone())));
        registry.register(Arc::new(Artic::new(client.clone())));
        registry.register(Arc::new(AlpineLinux::new(client.clone())));

        // ── Batch 2: More SearXNG translations ────────────
        registry.register(Arc::new(GitHub::new(client.clone())));
        registry.register(Arc::new(HackerNews::new(client.clone())));
        registry.register(Arc::new(DockerHub::new(client.clone())));
        registry.register(Arc::new(Npm::new(client.clone())));
        registry.register(Arc::new(CratesIo::new(client.clone())));
        registry.register(Arc::new(PyPI::new(client.clone())));
        registry.register(Arc::new(Reddit::new(client.clone())));
        registry.register(Arc::new(Dailymotion::new(client.clone())));
        registry.register(Arc::new(Deezer::new(client.clone())));
        registry.register(Arc::new(Ebay::new(client.clone())));
        registry.register(Arc::new(Imdb::new(client.clone())));
        registry.register(Arc::new(SoundCloud::new(client.clone())));
        registry.register(Arc::new(Flickr::new(client.clone())));

        // ── Batch 3: Even more SearXNG translations ───────
        registry.register(Arc::new(YouTube::new(client.clone())));
        registry.register(Arc::new(Spotify::new(client.clone())));
        registry.register(Arc::new(Crossref::new(client.clone())));
        registry.register(Arc::new(Lemmy::new(client.clone())));
        registry.register(Arc::new(Mastodon::new(client.clone())));
        registry.register(Arc::new(HuggingFace::new(client.clone())));
        registry.register(Arc::new(Goodreads::new(client.clone())));
        registry.register(Arc::new(BingNews::new(client.clone())));
        registry.register(Arc::new(BingImages::new(client.clone())));
        registry.register(Arc::new(BingVideos::new(client.clone())));
        registry.register(Arc::new(Genius::new(client.clone())));
        registry.register(Arc::new(GitLab::new(client.clone())));

        // ── Batch 4: Continuing SearXNG translations ──────
        registry.register(Arc::new(Yahoo::new(client.clone())));
        registry.register(Arc::new(Qwant::new(client.clone())));
        registry.register(Arc::new(Vimeo::new(client.clone())));
        registry.register(Arc::new(Unsplash::new(client.clone())));
        registry.register(Arc::new(SemanticScholar::new(client.clone())));
        registry.register(Arc::new(StackExchange::new(client.clone())));
        registry.register(Arc::new(Freesound::new(client.clone())));

        // ── Batch 5: More SearXNG translations ────────────
        registry.register(Arc::new(LeetX::new(client.clone())));
        registry.register(Arc::new(ApkMirror::new(client.clone())));
        registry.register(Arc::new(ArchLinux::new(client.clone())));
        registry.register(Arc::new(ArtStation::new(client.clone())));
        registry.register(Arc::new(Fdroid::new(client.clone())));

        // ── Batch 6: More SearXNG translations ────────────
        registry.register(Arc::new(AcFun::new(client.clone())));
        registry.register(Arc::new(Ansa::new(client.clone())));
        registry.register(Arc::new(BitChute::new(client.clone())));
        registry.register(Arc::new(Bpb::new(client.clone())));
        registry.register(Arc::new(Chefkoch::new(client.clone())));
        registry.register(Arc::new(Emojipedia::new(client.clone())));
        registry.register(Arc::new(FindThatMeme::new(client.clone())));
        registry.register(Arc::new(Fyyd::new(client.clone())));
        registry.register(Arc::new(Mixcloud::new(client.clone())));

        // ── Batch 7: More SearXNG translations ────────────
        registry.register(Arc::new(Bt4g::new(client.clone())));
        registry.register(Arc::new(Btdigg::new(client.clone())));
        registry.register(Arc::new(CachyOs::new(client.clone())));
        registry.register(Arc::new(CccMedia::new(client.clone())));
        registry.register(Arc::new(Destatis::new(client.clone())));

        // ── Batch 8: More SearXNG translations ────────────
        registry.register(Arc::new(Frinkiac::new(client.clone())));
        registry.register(Arc::new(Hex::new(client.clone())));
        registry.register(Arc::new(Ina::new(client.clone())));
        registry.register(Arc::new(Ipernity::new(client.clone())));
        registry.register(Arc::new(Devicons::new(client.clone())));

        // ── Batch 9: More SearXNG translations ────────────
        registry.register(Arc::new(AdobeStock::new(client.clone())));
        registry.register(Arc::new(AnnasArchive::new(client.clone())));
        registry.register(Arc::new(BaseSearch::new(client.clone())));
        registry.register(Arc::new(Digbt::new(client.clone())));

        // ── Batch 10: More SearXNG translations ───────────
        registry.register(Arc::new(Geizhals::new(client.clone())));
        registry.register(Arc::new(Grokipedia::new(client.clone())));
        registry.register(Arc::new(IlPost::new(client.clone())));
        registry.register(Arc::new(Loc::new(client.clone())));
        registry.register(Arc::new(MetaCpan::new(client.clone())));

        // ── Batch 11: More SearXNG translations ───────────
        registry.register(Arc::new(Duden::new(client.clone())));
        registry.register(Arc::new(Gitea::new(client.clone())));
        registry.register(Arc::new(LiveSpace::new(client.clone())));
        registry.register(Arc::new(MaterialIcons::new(client.clone())));
        registry.register(Arc::new(MediathekViewWeb::new(client.clone())));

        // ── Batch 12: More SearXNG translations ───────────
        registry.register(Arc::new(Iqiyi::new(client.clone())));
        registry.register(Arc::new(Jisho::new(client.clone())));
        registry.register(Arc::new(Lucide::new(client.clone())));
        registry.register(Arc::new(Mwmbl::new(client.clone())));
        registry.register(Arc::new(Nyaa::new(client.clone())));
        registry.register(Arc::new(Odysee::new(client.clone())));
        registry.register(Arc::new(SvgRepo::new(client.clone())));
        registry.register(Arc::new(Wallhaven::new(client.clone())));
        registry.register(Arc::new(Yep::new(client.clone())));

        // ── Batch 13: More SearXNG translations ───────────
        registry.register(Arc::new(PeerTube::new(client.clone())));
        registry.register(Arc::new(PkgGoDev::new(client.clone())));
        registry.register(Arc::new(Stract::new(client.clone())));
        registry.register(Arc::new(Tagesschau::new(client.clone())));
        registry.register(Arc::new(VoidLinux::new(client.clone())));
        registry.register(Arc::new(Rumble::new(client.clone())));
        registry.register(Arc::new(Pinterest::new(client.clone())));
        registry.register(Arc::new(PodcastIndex::new(client.clone())));
        registry.register(Arc::new(Photon::new(client.clone())));

        // ── Batch 14: More SearXNG translations ───────────
        registry.register(Arc::new(Moviepilot::new(client.clone())));
        registry.register(Arc::new(OpenLibrary::new(client.clone())));
        registry.register(Arc::new(SolidTorrents::new(client.clone())));
        registry.register(Arc::new(RottenTomatoes::new(client.clone())));
        registry.register(Arc::new(SepiaSearch::new(client.clone())));

        // ── Batch 15: More SearXNG translations ───────────
        registry.register(Arc::new(Openverse::new(client.clone())));
        registry.register(Arc::new(Tootfinder::new(client.clone())));
        registry.register(Arc::new(Searchcode::new(client.clone())));
        registry.register(Arc::new(TokyoToshokan::new(client.clone())));

        // ── Batch 16: Wired orphans + new engines ─────────
        registry.register(Arc::new(Imgur::new(client.clone())));
        registry.register(Arc::new(LibRs::new(client.clone())));
        registry.register(Arc::new(Kickass::new(client.clone())));
        registry.register(Arc::new(DeviantArt::new(client.clone())));
        registry.register(Arc::new(ThreeSixtySearchVideos::new(client.clone())));
        registry.register(Arc::new(Sourcehut::new(client.clone())));

        // ── Batch 17: More SearXNG translations ───────────
        registry.register(Arc::new(Chinaso::new(client.clone())));
        registry.register(Arc::new(FlickrNoapi::new(client.clone())));
        registry.register(Arc::new(Ahmia::new(client.clone())));
        registry.register(Arc::new(Naver::new(client.clone())));
        registry.register(Arc::new(RadioBrowser::new(client.clone())));

        // ── Batch 18: More SearXNG translations ───────────
        registry.register(Arc::new(Mojeek::new(client.clone())));
        registry.register(Arc::new(GooglePlay::new(client.clone())));
        registry.register(Arc::new(Yandex::new(client.clone())));

        // ── Batch 19: Wired orphans ───────────────────────
        registry.register(Arc::new(PirateBay::new(client.clone())));
        registry.register(Arc::new(OpenAlex::new(client.clone())));
        registry.register(Arc::new(RightDao::new(client.clone())));

        // ── Batch 20: More SearXNG translations ───────────
        registry.register(Arc::new(Sogou::new(client.clone())));
        registry.register(Arc::new(Quark::new(client.clone())));
        registry.register(Arc::new(WikiCommons::new(client.clone())));

        // ── Batch 21: API key + multi-module engines ──────
        registry.register(Arc::new(BraveApi::new(client.clone(), None)));
        registry.register(Arc::new(CoreEngine::new(client.clone(), None)));
        registry.register(Arc::new(Springer::new(client.clone(), None)));
        registry.register(Arc::new(Ads::new(client.clone(), None)));
        registry.register(Arc::new(Marginalia::new(client.clone(), None)));
        registry.register(Arc::new(DuckDuckGoDefinitions::new(client.clone())));
        registry.register(Arc::new(GoogleImages::new(client.clone())));
        registry.register(Arc::new(GoogleScholar::new(client.clone())));

        // ── Batch 22: Instance-URL + multi-module engines ─
        // Google multi-module engines (no config needed)
        registry.register(Arc::new(GoogleVideos::new(client.clone())));
        registry.register(Arc::new(GoogleNews::new(client.clone())));
        // Instance-URL engines (disabled by default, need base_url configuration)
        registry.register(Arc::new(Discourse::new(client.clone(), "", None, None)));
        registry.register(Arc::new(Invidious::new(client.clone(), "")));
        registry.register(Arc::new(Piped::new(client.clone(), "", "")));
        registry.register(Arc::new(MediaWikiEngine::new(client.clone(), "")));
        registry.register(Arc::new(ElasticsearchEngine::new(
            client.clone(),
            "",
            "",
            None,
            None,
        )));
        registry.register(Arc::new(MeilisearchEngine::new(
            client.clone(),
            "",
            "",
            None,
        )));
        registry.register(Arc::new(Doku::new(client.clone(), "")));
        registry.register(Arc::new(RecollEngine::new(client.clone(), "", "", "")));

        // ── Batch 23: Translation, dictionary, weather, maps, currency ─
        // Translation engines (configurable, disabled by default)
        registry.register(Arc::new(LibreTranslate::new(client.clone(), "", None)));
        registry.register(Arc::new(Lingva::new(client.clone(), "")));
        registry.register(Arc::new(DeepL::new(client.clone(), None)));
        // Dictionary & general engines (always enabled)
        registry.register(Arc::new(DictZone::new(client.clone())));
        registry.register(Arc::new(Wordnik::new(client.clone())));
        registry.register(Arc::new(CurrencyConvert::new(client.clone())));
        // Reverse image search
        registry.register(Arc::new(TinEye::new(client.clone())));
        // Map engines
        registry.register(Arc::new(OpenStreetMap::new(client.clone())));
        registry.register(Arc::new(AppleMaps::new(client.clone())));
        // Weather
        registry.register(Arc::new(DuckDuckGoWeather::new(client.clone())));

        // ── Batch 24: New engines ─────────────────────────
        registry.register(Arc::new(SteamStore::new(client.clone())));
        registry.register(Arc::new(Nvd::new(client.clone())));
        registry.register(Arc::new(MicrosoftLearn::new(client.clone())));
        registry.register(Arc::new(SearchcodeCode::new(client.clone())));
        registry.register(Arc::new(Repology::new(client.clone())));
        registry.register(Arc::new(Selfhst::new(client.clone())));

        // ── Batch 25: Sogou variants, Reuters, ScanR, PDBe ──
        registry.register(Arc::new(SogouVideos::new(client.clone())));
        registry.register(Arc::new(SogouImages::new(client.clone())));
        registry.register(Arc::new(SogouWechat::new(client.clone())));
        registry.register(Arc::new(Reuters::new(client.clone())));
        registry.register(Arc::new(ScanrStructures::new(client.clone())));
        registry.register(Arc::new(Pdbe::new(client.clone())));

        // ── Batch 26: MRS, SensCritique, Yahoo News, OpenClipart, UXWing, 1x ──
        registry.register(Arc::new(Mrs::new(client.clone(), "")));
        registry.register(Arc::new(SensCritique::new(client.clone())));
        registry.register(Arc::new(YahooNews::new(client.clone())));
        registry.register(Arc::new(Openclipart::new(client.clone())));
        registry.register(Arc::new(Uxwing::new(client.clone())));
        registry.register(Arc::new(Www1x::new(client.clone())));

        // ── Batch 29: YouTube/Wolfram variants, ADS, Presearch ──
        registry.register(Arc::new(YoutubeApi::new(client.clone(), None)));
        registry.register(Arc::new(YoutubeNoapi::new(client.clone())));
        registry.register(Arc::new(WolframAlphaApi::new(client.clone(), None)));
        registry.register(Arc::new(WolframAlphaNoapi::new(client.clone())));
        registry.register(Arc::new(AstrophysicsDataSystem::new(client.clone(), None)));
        registry.register(Arc::new(Presearch::new(client.clone())));

        // ── Batch 27: Yandex Music, Pixabay, Niconico, PubMed, GitHub Code, Z-Library ──
        registry.register(Arc::new(YandexMusic::new(client.clone())));
        registry.register(Arc::new(Pixabay::new(client.clone())));
        registry.register(Arc::new(Niconico::new(client.clone())));
        registry.register(Arc::new(Pubmed::new(client.clone())));
        registry.register(Arc::new(GithubCode::new(client.clone(), None)));
        registry.register(Arc::new(Zlibrary::new(client.clone())));

        // ── Batch 28: Translation, weather, search engines ──
        registry.register(Arc::new(Translated::new(client.clone())));
        registry.register(Arc::new(Mozhi::new(client.clone(), "")));
        registry.register(Arc::new(Wttr::new(client.clone())));
        registry.register(Arc::new(OpenMeteo::new(client.clone())));
        registry.register(Arc::new(Seznam::new(client.clone())));
        registry.register(Arc::new(Startpage::new(client.clone())));

        // ── Batch 30: Pexels, Pixiv, DDG Extra, 360 Search, YaCy, Torznab ──
        registry.register(Arc::new(DuckDuckGoExtra::new(client.clone())));
        registry.register(Arc::new(Pexels::new(client.clone(), None)));
        registry.register(Arc::new(Pixiv::new(client.clone())));
        registry.register(Arc::new(ThreeSixtySearch::new(client.clone())));
        registry.register(Arc::new(Yacy::new(client.clone(), "")));
        registry.register(Arc::new(Torznab::new(client.clone(), "", None)));

        // ── Batch 31: Self-hosted, Wikidata, PDIA, SearXNG federation ──
        registry.register(Arc::new(Wikidata::new(client.clone())));
        registry.register(Arc::new(PublicDomainImageArchive::new(client.clone())));
        registry.register(Arc::new(Solr::new(client.clone(), "")));
        registry.register(Arc::new(OpenSemantic::new(client.clone(), "")));
        registry.register(Arc::new(SearxEngine::new(client.clone(), "")));
        registry.register(Arc::new(TubeArchivist::new(client.clone(), "", None)));

        // ── Batch 32: AI/Cloud: Ollama, Cloudflare AI, Azure Search ──
        registry.register(Arc::new(Ollama::new(client.clone(), "", None)));
        registry.register(Arc::new(CloudflareAi::new(client.clone(), None, None, None)));
        registry.register(Arc::new(Azure::new(client.clone(), "", None, None)));

        registry
    }

    /// Register a new engine.
    pub fn register(&mut self, engine: Arc<dyn SearchEngine>) {
        let name = engine.metadata().name.into_owned();
        self.engines.insert(name, engine);
    }

    /// Get an engine by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn SearchEngine>> {
        self.engines.get(name).map(|r| Arc::clone(&r))
    }

    /// Get all enabled engines for a given category.
    pub fn engines_for_category(&self, category: &SearchCategory) -> Vec<Arc<dyn SearchEngine>> {
        self.engines
            .iter()
            .filter(|e| e.metadata().enabled && e.metadata().categories.contains(category))
            .map(|r| Arc::clone(&r))
            .collect()
    }

    /// List all registered engine names.
    pub fn engine_names(&self) -> Vec<String> {
        self.engines.iter().map(|r| r.key().clone()).collect()
    }

    /// Number of registered engines.
    pub fn count(&self) -> usize {
        self.engines.len()
    }
}

impl Default for EngineRegistry {
    fn default() -> Self {
        Self::new()
    }
}
