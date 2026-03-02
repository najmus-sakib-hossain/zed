//! # metasearch-engine
//!
//! Concrete search engine implementations.
//! Each engine scrapes or queries a search provider
//! and returns normalized `SearchResult` items.
//!
//! ## Original engines (4):
//! - Google, DuckDuckGo, Brave, Wikipedia
//!
//! ## Batch 1 — SearXNG translations (10):
//! - Bing, arXiv, Ask, Bandcamp, Baidu, 9GAG,
//!   Apple App Store, Bilibili, Art Institute of Chicago, Alpine Linux
//!
//! ## Batch 2 — More SearXNG translations (13):
//! - GitHub, Hacker News, Docker Hub, npm, crates.io, PyPI,
//!   Reddit, Dailymotion, Deezer, eBay, IMDb, SoundCloud, Flickr
//!
//! ## Batch 3 — Even more SearXNG translations (12):
//! - YouTube, Spotify, Crossref, Lemmy, Mastodon, Hugging Face,
//!   Goodreads, Bing News, Bing Images, Bing Videos, Genius, GitLab
//!
//! ## Batch 4 — Continuing SearXNG translations (7):
//! - Yahoo, Qwant, Vimeo, Unsplash, Semantic Scholar,
//!   StackExchange, Freesound
//!
//! ## Batch 5 — More SearXNG translations (5):
//! - 1337x, APKMirror, Arch Linux Wiki, ArtStation, F-Droid
//!
//! ## Batch 6 — More SearXNG translations (9):
//! - AcFun, ANSA, BitChute, BPB, Chefkoch, Emojipedia,
//!   FindThatMeme, Fyyd, Mixcloud
//!
//! ## Batch 7 — More SearXNG translations (5):
//! - BT4G, BTDigg, CachyOS, CCC Media, DeStatis
//!
//! ## Batch 8 — More SearXNG translations (5):
//! - Frinkiac, Hex.pm, INA, Ipernity, Devicons
//!
//! ## Batch 9 — More SearXNG translations (4):
//! - Adobe Stock, Anna's Archive, BASE, DigBT
//!
//! ## Batch 10 — More SearXNG translations (5):
//! - Geizhals, Grokipedia, Il Post, Library of Congress, MetaCPAN
//!
//! ## Batch 11 — More SearXNG translations (5):
//! - Duden, Gitea, LiveSpace, Material Icons, MediathekViewWeb
//!
//! ## Batch 12 — More SearXNG translations (9):
//! - iQiyi, Jisho, Lucide, Mwmbl, Nyaa, Odysee, SVG Repo, Wallhaven, Yep
//!
//! ## Batch 13 — More SearXNG translations (9):
//! - PeerTube, pkg.go.dev, Stract, Tagesschau, Void Linux,
//!   Rumble, Pinterest, Podcast Index, Photon
//!
//! ## Batch 14 — More SearXNG translations (5):
//! - Moviepilot, Open Library, SolidTorrents, Rotten Tomatoes, SepiaSearch
//!
//! ## Batch 15 — More SearXNG translations (4):
//! - Openverse, Tootfinder, Searchcode, Tokyo Toshokan
//!
//! ## Batch 16 — Wired orphans + new engines (6):
//! - Imgur, lib.rs, Kickass Torrents, DeviantArt,
//!   360 Search Videos, SourceHut
//!
//! ## Batch 17 — More SearXNG translations (5):
//! - Chinaso, Flickr (no API), Ahmia, Naver, Radio Browser
//!
//! ## Batch 18 — More SearXNG translations (3):
//! - Mojeek, Google Play, Yandex
//!
//! ## Batch 19 — Wired orphans (2):
//! - PirateBay, OpenAlex
//!
//! ## Batch 20 — More SearXNG translations (3):
//! - Sogou, Quark, Wikimedia Commons
//!
//! ## Batch 21 — API key + multi-module engines (8):
//! - Brave API, CORE, Springer Nature, NASA ADS, Marginalia,
//!   DuckDuckGo Definitions, Google Images, Google Scholar
//!
//! ## Batch 22 — Instance-URL + multi-module engines (10):
//! - Google Videos, Google News, Discourse, Invidious, Piped,
//!   MediaWiki, Elasticsearch, MeiliSearch, DokuWiki, Recoll
//!
//! ## Batch 23 — Translation, dictionary, weather, maps, currency (10):
//! - LibreTranslate, Lingva, DictZone, DeepL, CurrencyConvert,
//!   Wordnik, TinEye, OpenStreetMap, Apple Maps, DuckDuckGo Weather
//!
//! ## Batch 33 — Database engines (6, optional features):
//! - SQLite, PostgreSQL, MySQL, MariaDB, MongoDB, Valkey/Redis
//! - Enable with cargo features: `sqlite`, `postgres`, `mysql`, `mongodb_feature`, `redis`

// Original engines
pub mod brave;
pub mod duckduckgo;
pub mod google;
pub mod wikipedia;

// Batch 1
pub mod alpinelinux;
pub mod apple_app_store;
pub mod artic;
pub mod arxiv;
pub mod ask;
pub mod baidu;
pub mod bandcamp;
pub mod bilibili;
pub mod bing;
pub mod nine_gag;

// Batch 2
pub mod crates_io;
pub mod dailymotion;
pub mod deezer;
pub mod docker_hub;
pub mod ebay;
pub mod flickr;
pub mod github_engine;
pub mod hackernews;
pub mod imdb;
pub mod npm;
pub mod pypi;
pub mod reddit;
pub mod soundcloud;

// Batch 3
pub mod bing_images;
pub mod bing_news;
pub mod bing_videos;
pub mod crossref;
pub mod genius;
pub mod gitlab;
pub mod goodreads;
pub mod huggingface;
pub mod lemmy;
pub mod mastodon;
pub mod spotify;
pub mod youtube;

// Batch 4
pub mod freesound;
pub mod qwant;
pub mod semantic_scholar;
pub mod stackexchange;
pub mod unsplash;
pub mod vimeo;
pub mod yahoo;

// Batch 5
pub mod apkmirror;
pub mod archlinux;
pub mod artstation;
pub mod fdroid;
pub mod leet_x;

// Batch 6
pub mod acfun;
pub mod ansa;
pub mod bitchute;
pub mod bpb;
pub mod chefkoch;
pub mod emojipedia;
pub mod findthatmeme;
pub mod fyyd;
pub mod mixcloud;

// Batch 7
pub mod bt4g;
pub mod btdigg;
pub mod cachy_os;
pub mod ccc_media;
pub mod destatis;

// Batch 8
pub mod devicons;
pub mod frinkiac;
pub mod hex;
pub mod ina;
pub mod ipernity;

// Batch 9
pub mod adobe_stock;
pub mod annas_archive;
pub mod base_search;
pub mod digbt;

// Batch 10
pub mod geizhals;
pub mod grokipedia;
pub mod il_post;
pub mod loc;
pub mod metacpan;

// Batch 11
pub mod duden;
pub mod gitea;
pub mod livespace;
pub mod material_icons;
pub mod mediathekviewweb;

// Batch 12
pub mod iqiyi;
pub mod jisho;
pub mod lucide;
pub mod mwmbl;
pub mod nyaa;
pub mod odysee;
pub mod svgrepo;
pub mod wallhaven;
pub mod yep;

// Batch 13
pub mod peertube;
pub mod photon;
pub mod pinterest;
pub mod pkg_go_dev;
pub mod podcastindex;
pub mod rumble;
pub mod stract;
pub mod tagesschau;
pub mod voidlinux;

// Batch 14
pub mod moviepilot;
pub mod openlibrary;
pub mod rottentomatoes;
pub mod sepiasearch;
pub mod solidtorrents;

// Batch 15
pub mod openverse;
pub mod searchcode;
pub mod tokyotoshokan;
pub mod tootfinder;

// Batch 16 (wired orphans + new)
pub mod deviantart;
pub mod imgur;
pub mod kickass;
pub mod lib_rs;
pub mod sourcehut;
pub mod three_sixty_search_videos;

// Batch 17
pub mod ahmia;
pub mod chinaso;
pub mod flickr_noapi;
pub mod naver;
pub mod radio_browser;

// Batch 18
pub mod google_play;
pub mod mojeek;
pub mod yandex;

// Batch 19 (wired orphans)
pub mod openalex;
pub mod piratebay;
pub mod rightdao;

// Batch 20
pub mod quark;
pub mod sogou;
pub mod wikicommons;

// Batch 21 (API key + multi-module engines)
pub mod ads;
pub mod braveapi;
pub mod core_engine;
pub mod duckduckgo_definitions;
pub mod google_images;
pub mod google_scholar;
pub mod marginalia;
pub mod springer;

// Batch 22 (instance-URL + multi-module engines)
pub mod discourse;
pub mod doku;
pub mod elasticsearch_engine;
pub mod google_news;
pub mod google_videos;
pub mod invidious;
pub mod mediawiki_engine;
pub mod meilisearch_engine;
pub mod piped;
pub mod recoll_engine;

// Batch 23 (translation, dictionary, weather, maps, currency)
pub mod apple_maps;
pub mod currency_convert;
pub mod deepl;
pub mod dictzone;
pub mod duckduckgo_weather;
pub mod libretranslate;
pub mod lingva;
pub mod openstreetmap;
pub mod tineye;
pub mod wordnik;

// Batch 24 (new engines)
pub mod microsoft_learn;
pub mod nvd;
pub mod repology;
pub mod searchcode_code;
pub mod selfhst;
pub mod steam;

// Batch 25 (Sogou variants, Reuters, ScanR, PDBe)
pub mod pdbe;
pub mod reuters;
pub mod scanr_structures;
pub mod sogou_images;
pub mod sogou_videos;
pub mod sogou_wechat;

// Batch 26 (MRS, SensCritique, Yahoo News, OpenClipart, UXWing, 1x)
pub mod mrs;
pub mod openclipart;
pub mod senscritique;
pub mod uxwing;
pub mod www1x;
pub mod yahoo_news;

// Batch 27 (Yandex Music, Pixabay, Niconico, PubMed, GitHub Code, Z-Library)
pub mod github_code;
pub mod niconico;
pub mod pixabay;
pub mod pubmed;
pub mod yandex_music;
pub mod zlibrary;

// Batch 28 (Translation, weather, search engines)
pub mod mozhi;
pub mod open_meteo;
pub mod seznam;
pub mod startpage;
pub mod translated;
pub mod wttr;

// Batch 29 (YouTube/Wolfram variants, ADS, Presearch)
pub mod astrophysics_data_system;
pub mod presearch;
pub mod wolframalpha_api;
pub mod wolframalpha_noapi;
pub mod youtube_api;
pub mod youtube_noapi;

// Batch 30 (Pexels, Pixiv, DDG Extra, 360 Search, YaCy, Torznab)
pub mod duckduckgo_extra;
pub mod pexels;
pub mod pixiv;
pub mod three_sixty_search;
pub mod torznab;
pub mod yacy;

// Batch 31 (Self-hosted, Wikidata, PDIA, SearXNG federation)
pub mod opensemantic;
pub mod public_domain_image_archive;
pub mod searx_engine;
pub mod solr;
pub mod tubearchivist;
pub mod wikidata;

// Batch 32 (AI/Cloud: Ollama, Cloudflare AI, Azure Search)
pub mod azure;
pub mod cloudflareai;
pub mod ollama;

// Batch 33 (Database engines — offline, require database drivers)
pub mod mariadb_engine;
pub mod mongodb_engine;
pub mod mysql_engine;
pub mod postgres_engine;
pub mod sqlite_engine;
pub mod valkey_engine;

pub mod registry;

pub use registry::EngineRegistry;
