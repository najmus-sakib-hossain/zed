//! Key abbreviation dictionary for LLM and Human format conversion
//!
//! This module provides bidirectional key abbreviation mappings for converting
//! between token-efficient LLM format and human-readable format.
//! Contains 100+ mappings for comprehensive domain coverage.

use std::collections::HashMap;

/// Bidirectional key abbreviation dictionary
///
/// Provides mappings between abbreviated keys (for LLM format) and
/// full key names (for Human format), with context-aware expansion
/// for ambiguous abbreviations.
#[derive(Debug, Clone)]
pub struct AbbrevDict {
    /// Short → Full (for expansion)
    global: HashMap<&'static str, &'static str>,
    /// Context-specific expansions: (abbrev, context) → full
    contextual: HashMap<(&'static str, &'static str), &'static str>,
    /// Full → Short (for compression)
    reverse: HashMap<&'static str, &'static str>,
}

impl AbbrevDict {
    /// Create dictionary with all standard mappings (100+ entries)
    pub fn new() -> Self {
        let mut global = HashMap::new();
        let mut reverse = HashMap::new();
        let mut contextual = HashMap::new();

        // Helper to add bidirectional mapping
        let mut add = |abbrev: &'static str, full: &'static str| {
            global.insert(abbrev, full);
            reverse.insert(full, abbrev);
        };

        // ============================================================
        // IDENTITY & NAMING (15 mappings)
        // ============================================================
        add("id", "id");
        add("nm", "name");
        add("tt", "title");
        add("ds", "description");
        add("lb", "label");
        add("al", "alias");
        add("uid", "unique_id");
        add("uuid", "uuid");
        add("slug", "slug");
        add("hdl", "handle");
        add("nick", "nickname");
        add("disp", "display_name");
        add("abbr", "abbreviation");
        add("code", "code");
        add("ref", "reference");

        // ============================================================
        // STATE & STATUS (20 mappings)
        // ============================================================
        add("st", "status");
        add("ac", "active");
        add("en", "enabled");
        add("vs", "visible");
        add("lk", "locked");
        add("ar", "archived");
        add("dl", "deleted");
        add("cp", "completed");
        add("pn", "pending");
        add("pub", "published");
        add("drft", "draft");
        add("appr", "approved");
        add("rej", "rejected");
        add("susp", "suspended");
        add("exp", "expired");
        add("canc", "cancelled");
        add("proc", "processing");
        add("fail", "failed");
        add("succ", "success");
        add("rdy", "ready");

        // ============================================================
        // TIMESTAMPS & DATES (20 mappings)
        // ============================================================
        add("cr", "created");
        add("up", "updated");
        add("dt", "date");
        add("tm", "time");
        add("ts", "timestamp");
        add("ex", "expires");
        add("du", "duration");
        add("yr", "year");
        add("mo", "month");
        add("dy", "day");
        add("hr", "hour");
        add("mn", "minute");
        add("sec", "second");
        add("ms", "millisecond");
        add("tz", "timezone");
        add("utc", "utc");
        add("strt", "start");
        add("end", "end");
        add("schd", "scheduled");
        add("dln", "deadline");

        // ============================================================
        // METRICS & NUMBERS (25 mappings)
        // ============================================================
        add("ct", "count");
        add("tl", "total");
        add("am", "amount");
        add("pr", "price");
        add("qt", "quantity");
        add("km", "kilometers");
        add("mi", "miles");
        add("el", "elevation");
        add("rt", "rating");
        add("sc", "score");
        add("rk", "rank");
        add("pct", "percent");
        add("avg", "average");
        add("min", "minimum");
        add("max", "maximum");
        add("sum", "sum");
        add("med", "median");
        add("std", "standard_deviation");
        add("var", "variance");
        add("idx", "index");
        add("pos", "position");
        add("ord", "order");
        add("seq", "sequence");
        add("num", "number");
        // "vl" -> "value" already defined in classification section

        // ============================================================
        // DIMENSIONS & MEASUREMENTS (15 mappings)
        // ============================================================
        add("wd", "width");
        add("ht", "height");
        add("sz", "size");
        add("len", "length");
        add("dp", "depth");
        add("wt", "weight");
        add("vol", "volume");
        add("area", "area");
        add("rad", "radius");
        add("dia", "diameter");
        add("cap", "capacity");
        add("res", "resolution");
        add("dpi", "dpi");
        add("asp", "aspect_ratio");
        add("scl", "scale");

        // ============================================================
        // WEB & NETWORKING (20 mappings)
        // ============================================================
        add("ur", "url");
        add("pt", "path");
        add("lnk", "link");
        add("src", "source");
        add("dst", "destination");
        add("dom", "domain");
        add("api", "api");
        add("ep", "endpoint");
        add("mth", "method");
        add("hdr", "header");
        add("bdy", "body");
        add("qry", "query");
        add("prm", "param");
        add("rsp", "response");
        add("req", "request");
        add("ip", "ip_address");
        add("port", "port");
        add("prot", "protocol");
        add("ssl", "ssl");
        add("cert", "certificate");

        // ============================================================
        // CONTACT & PERSONAL (15 mappings)
        // ============================================================
        add("em", "email");
        add("ph", "phone");
        add("ad", "address");
        add("fn", "first_name");
        add("lnm", "last_name");
        add("cmp", "company");
        add("dob", "date_of_birth");
        add("gen", "gender");
        add("bio", "biography");
        add("avt", "avatar");
        add("prof", "profile");
        add("pref", "preferences");
        add("lang", "language");
        add("cntry", "country_code");
        add("mob", "mobile");

        // ============================================================
        // LOCATION & GEO (15 mappings)
        // ============================================================
        add("cy", "city");
        add("co", "country");
        add("rg", "region");
        add("zp", "zipcode");
        add("la", "latitude");
        add("lo", "longitude");
        add("loc", "location");
        add("geo", "geo");
        add("addr", "street_address");
        add("st2", "address_line_2");
        add("prov", "province");
        add("dist", "district");
        add("bldg", "building");
        add("flr", "floor");
        add("unit", "unit");

        // ============================================================
        // VISUAL & MEDIA (15 mappings)
        // ============================================================
        add("cl", "color");
        add("bg", "background");
        add("fg", "foreground");
        add("im", "image");
        add("ic", "icon");
        add("th", "thumbnail");
        add("vid", "video");
        add("aud", "audio");
        add("fmt", "format");
        add("mime", "mime_type");
        add("ext", "extension");
        add("fsize", "file_size");
        // "du" -> "duration" already defined in timestamps section
        add("bps", "bitrate");
        add("fps", "framerate");

        // ============================================================
        // RELATIONS & HIERARCHY (20 mappings)
        // ============================================================
        add("pa", "parent");
        add("ch", "children");
        add("us", "user");
        add("ow", "owner");
        add("au", "author");
        add("ed", "editor");
        add("rv", "reviewer");
        add("asg", "assignee");
        add("mb", "member");
        add("gp", "group");
        add("tea", "team");
        add("org", "organization");
        add("dept", "department");
        add("mgr", "manager");
        add("sup", "supervisor");
        add("sub", "subordinate");
        add("peer", "peer");
        add("anc", "ancestor");
        add("desc", "descendant");
        add("sib", "sibling");

        // ============================================================
        // CLASSIFICATION & TAXONOMY (15 mappings)
        // ============================================================
        add("ca", "category");
        add("tg", "tags");
        add("tp", "type");
        add("vl", "value");
        add("ky", "key");
        add("md", "mode");
        add("lv", "level");
        add("pri", "priority");
        add("vr", "version");
        add("cls", "class");
        add("kind", "kind");
        add("grp", "group_type");
        add("tier", "tier");
        add("rank", "ranking");
        add("flag", "flag");

        // ============================================================
        // PROJECT & WORKSPACE (15 mappings)
        // ============================================================
        add("ws", "workspace");
        add("repo", "repository");
        add("cont", "container");
        add("ci", "ci_cd");
        add("eds", "editors");
        add("proj", "project");
        add("env", "environment");
        add("cfg", "config");
        add("sett", "settings");
        add("opt", "options");
        add("feat", "feature");
        add("mod", "module");
        add("pkg", "package");
        add("dep", "dependency");
        add("lib", "library");

        // ============================================================
        // COMMERCE & FINANCE (20 mappings)
        // ============================================================
        add("sk", "sku");
        add("cu", "customer");
        add("sh", "shipping");
        add("pd", "paid");
        add("inv", "invoice");
        add("prd", "product");
        add("dsc", "discount");
        add("tx", "tax");
        add("curr", "currency");
        add("bal", "balance");
        add("cred", "credit");
        add("deb", "debit");
        add("fee", "fee");
        add("sub", "subtotal");
        add("grt", "grand_total");
        add("pay", "payment");
        add("refnd", "refund");
        add("cart", "cart");
        add("chk", "checkout");
        add("bill", "billing");

        // ============================================================
        // CONTENT & TEXT (15 mappings)
        // ============================================================
        add("txt", "text");
        add("msg", "message");
        add("cmt", "comment");
        add("nt", "note");
        add("cnt", "content");
        // "bdy" -> "body" already defined in web section
        // "hdr" -> "header" already defined in web section
        add("ft", "footer");
        add("para", "paragraph");
        add("sect", "section");
        add("chap", "chapter");
        add("art", "article");
        add("post", "post");
        add("reply", "reply");
        add("subj", "subject");

        // ============================================================
        // SECURITY & AUTH (15 mappings)
        // ============================================================
        add("pwd", "password");
        add("hash", "hash");
        add("salt", "salt");
        add("tok", "token");
        add("sess", "session");
        add("perm", "permission");
        add("role", "role");
        add("auth", "authorization");
        add("acl", "access_control");
        add("enc", "encrypted");
        add("sig", "signature");
        add("key", "api_key");
        add("sec", "secret");
        add("2fa", "two_factor");
        add("otp", "one_time_password");

        // ============================================================
        // DATA & STORAGE (10 mappings)
        // ============================================================
        add("db", "database");
        add("tbl", "table");
        add("col", "column");
        add("row", "row");
        add("rec", "record");
        add("fld", "field");
        add("blob", "blob");
        add("json", "json");
        add("xml", "xml");
        add("csv", "csv");

        // ============================================================
        // LINT & CODE QUALITY (10 mappings)
        // ============================================================
        add("sev", "severity");
        add("fix", "fixable");
        add("recom", "recommended");
        add("fmtr", "formatter");
        add("pfx", "prefix");
        // Note: "category" is already mapped to "ca" in Classification section
        add("docs", "documentation");
        add("warn", "warning");
        add("err", "error");
        add("lint", "linter");

        // End the scope of `add` closure to release the mutable borrow

        // === Short aliases (expand only, don't affect compression) ===
        // "v" expands to "version" but "version" compresses to "vr"
        global.insert("v", "version");
        global.insert("n", "name");
        global.insert("d", "date");
        global.insert("t", "type");
        global.insert("s", "status");

        // === Context-aware expansions for ambiguous single-letter keys ===
        // 's' expansions
        contextual.insert(("s", "hikes"), "sunny");
        contextual.insert(("s", "weather"), "sunny");
        contextual.insert(("s", "orders"), "status");
        contextual.insert(("s", "tasks"), "status");
        contextual.insert(("s", "config"), "season");
        contextual.insert(("s", "default"), "status");

        // 'w' expansions
        contextual.insert(("w", "hikes"), "with");
        contextual.insert(("w", "images"), "width");
        contextual.insert(("w", "products"), "weight");
        contextual.insert(("w", "default"), "width");

        // 't' expansions
        contextual.insert(("t", "config"), "task");
        contextual.insert(("t", "products"), "type");
        contextual.insert(("t", "events"), "time");
        contextual.insert(("t", "default"), "type");

        // 'l' expansions
        contextual.insert(("l", "geo"), "location");
        contextual.insert(("l", "maps"), "location");
        contextual.insert(("l", "text"), "length");
        contextual.insert(("l", "default"), "location");

        // 'n' expansions
        contextual.insert(("n", "users"), "name");
        contextual.insert(("n", "items"), "name");
        contextual.insert(("n", "math"), "number");
        contextual.insert(("n", "default"), "name");

        // 'd' expansions
        contextual.insert(("d", "calendar"), "date");
        contextual.insert(("d", "events"), "date");
        contextual.insert(("d", "items"), "description");
        contextual.insert(("d", "default"), "date");

        // 'c' expansions
        contextual.insert(("c", "metrics"), "count");
        contextual.insert(("c", "items"), "category");
        contextual.insert(("c", "visual"), "color");
        contextual.insert(("c", "default"), "count");

        // 'v' expansions
        contextual.insert(("v", "data"), "value");
        contextual.insert(("v", "software"), "version");
        contextual.insert(("v", "default"), "version");

        // 'p' expansions
        contextual.insert(("p", "commerce"), "price");
        contextual.insert(("p", "tasks"), "priority");
        contextual.insert(("p", "files"), "path");
        contextual.insert(("p", "default"), "price");

        // 'a' expansions
        contextual.insert(("a", "metrics"), "amount");
        contextual.insert(("a", "users"), "author");
        contextual.insert(("a", "geo"), "address");
        contextual.insert(("a", "default"), "amount");

        // 'e' expansions
        contextual.insert(("e", "contact"), "email");
        contextual.insert(("e", "events"), "end");
        contextual.insert(("e", "status"), "enabled");
        contextual.insert(("e", "default"), "email");

        // 'u' expansions
        contextual.insert(("u", "web"), "url");
        contextual.insert(("u", "auth"), "user");
        contextual.insert(("u", "time"), "updated");
        contextual.insert(("u", "default"), "user");

        Self {
            global,
            contextual,
            reverse,
        }
    }

    /// Expand abbreviated key to full name
    ///
    /// Uses context-aware expansion for ambiguous single-letter keys.
    /// Falls back to global dictionary, then returns original if not found.
    pub fn expand(&self, abbrev: &str, context: &str) -> String {
        // First try context-specific expansion
        if let Some(&full) = self.contextual.get(&(abbrev, context)) {
            return full.to_string();
        }

        // Try default context for single-letter keys
        if abbrev.len() == 1 {
            if let Some(&full) = self.contextual.get(&(abbrev, "default")) {
                return full.to_string();
            }
        }

        // Fall back to global dictionary
        if let Some(&full) = self.global.get(abbrev) {
            return full.to_string();
        }

        // Return original if not found
        abbrev.to_string()
    }

    /// Compress full key to abbreviation
    ///
    /// Uses the shortest unambiguous abbreviation from the dictionary.
    /// Returns original if not found.
    pub fn compress(&self, full: &str) -> String {
        if let Some(&abbrev) = self.reverse.get(full) {
            return abbrev.to_string();
        }

        // Return original if not found
        full.to_string()
    }

    /// Check if an abbreviation exists in the dictionary
    pub fn has_abbrev(&self, abbrev: &str) -> bool {
        self.global.contains_key(abbrev)
    }

    /// Check if a full key exists in the dictionary
    pub fn has_full(&self, full: &str) -> bool {
        self.reverse.contains_key(full)
    }

    /// Get all global abbreviation mappings
    pub fn global_mappings(&self) -> &HashMap<&'static str, &'static str> {
        &self.global
    }

    /// Get the number of global mappings
    pub fn len(&self) -> usize {
        self.global.len()
    }

    /// Check if the dictionary is empty
    pub fn is_empty(&self) -> bool {
        self.global.is_empty()
    }

    /// Get compression ratio for a key (1.0 = no savings, <1.0 = savings)
    pub fn compression_ratio(&self, full: &str) -> f64 {
        let compressed = self.compress(full);
        compressed.len() as f64 / full.len() as f64
    }

    /// Get average compression ratio across all mappings
    pub fn average_compression_ratio(&self) -> f64 {
        if self.reverse.is_empty() {
            return 1.0;
        }
        let total: f64 = self
            .reverse
            .iter()
            .map(|(full, abbrev)| abbrev.len() as f64 / full.len() as f64)
            .sum();
        total / self.reverse.len() as f64
    }
}

impl Default for AbbrevDict {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abbrev_dict_has_100_plus_mappings() {
        let dict = AbbrevDict::new();
        assert!(
            dict.len() >= 100,
            "Dictionary should have at least 100 mappings, has {}",
            dict.len()
        );
    }

    #[test]
    fn test_expand_basic() {
        let dict = AbbrevDict::new();

        assert_eq!(dict.expand("nm", ""), "name");
        assert_eq!(dict.expand("tt", ""), "title");
        assert_eq!(dict.expand("ds", ""), "description");
        assert_eq!(dict.expand("st", ""), "status");
        assert_eq!(dict.expand("cr", ""), "created");
        assert_eq!(dict.expand("up", ""), "updated");
        assert_eq!(dict.expand("pr", ""), "price");
        assert_eq!(dict.expand("qt", ""), "quantity");
        assert_eq!(dict.expand("em", ""), "email");
        assert_eq!(dict.expand("ur", ""), "url");
    }

    #[test]
    fn test_expand_context_aware() {
        let dict = AbbrevDict::new();

        // 's' in different contexts
        assert_eq!(dict.expand("s", "hikes"), "sunny");
        assert_eq!(dict.expand("s", "orders"), "status");
        assert_eq!(dict.expand("s", "config"), "season");

        // 'w' in different contexts
        assert_eq!(dict.expand("w", "hikes"), "with");
        assert_eq!(dict.expand("w", "images"), "width");
        assert_eq!(dict.expand("w", "products"), "weight");

        // 't' in different contexts
        assert_eq!(dict.expand("t", "config"), "task");
        assert_eq!(dict.expand("t", "products"), "type");
        assert_eq!(dict.expand("t", "events"), "time");
    }

    #[test]
    fn test_compress_basic() {
        let dict = AbbrevDict::new();

        assert_eq!(dict.compress("name"), "nm");
        assert_eq!(dict.compress("title"), "tt");
        assert_eq!(dict.compress("description"), "ds");
        assert_eq!(dict.compress("status"), "st");
        assert_eq!(dict.compress("created"), "cr");
        assert_eq!(dict.compress("updated"), "up");
        assert_eq!(dict.compress("email"), "em");
        assert_eq!(dict.compress("url"), "ur");
    }

    #[test]
    fn test_unknown_key_passthrough() {
        let dict = AbbrevDict::new();

        // Unknown abbreviations pass through unchanged
        assert_eq!(dict.expand("xyz", ""), "xyz");
        assert_eq!(dict.expand("unknown_key", ""), "unknown_key");

        // Unknown full keys pass through unchanged
        assert_eq!(dict.compress("xyz"), "xyz");
        assert_eq!(dict.compress("unknown_key"), "unknown_key");
    }

    #[test]
    fn test_round_trip_global() {
        let dict = AbbrevDict::new();

        // For all global mappings, compress then expand should return original
        for (&abbrev, &full) in dict.global_mappings() {
            let compressed = dict.compress(full);
            let expanded = dict.expand(&compressed, "");
            assert_eq!(expanded, full, "Round-trip failed for {} -> {}", abbrev, full);
        }
    }

    #[test]
    fn test_round_trip_reverse() {
        let dict = AbbrevDict::new();

        // For all global mappings, check that expand then compress returns a VALID abbrev
        // The compressed form may not be the same as original if multiple abbreviations
        // map to the same expanded form (e.g., "ca" and "cat" both relate to category)
        // Skip single-letter keys as they use contextual expansion which may differ
        for (&abbrev, &_full) in dict.global_mappings() {
            if abbrev.len() == 1 {
                continue; // Single-letter keys use contextual expansion
            }
            let expanded = dict.expand(abbrev, "");
            let compressed = dict.compress(&expanded);
            // The compressed form should expand back to the same value
            let re_expanded = dict.expand(&compressed, "");
            assert_eq!(
                expanded, re_expanded,
                "Reverse round-trip failed for {}: {} -> {} -> {} -> {}",
                abbrev, abbrev, expanded, compressed, re_expanded
            );
        }
    }

    #[test]
    fn test_new_domain_abbreviations() {
        let dict = AbbrevDict::new();

        // Security domain
        assert_eq!(dict.expand("pwd", ""), "password");
        assert_eq!(dict.expand("tok", ""), "token");
        assert_eq!(dict.expand("sess", ""), "session");

        // Commerce domain
        assert_eq!(dict.expand("curr", ""), "currency");
        assert_eq!(dict.expand("bal", ""), "balance");
        assert_eq!(dict.expand("refnd", ""), "refund");

        // Data domain
        assert_eq!(dict.expand("db", ""), "database");
        assert_eq!(dict.expand("tbl", ""), "table");
        assert_eq!(dict.expand("col", ""), "column");
    }

    #[test]
    fn test_compression_ratio() {
        let dict = AbbrevDict::new();

        // "description" (11 chars) -> "ds" (2 chars) = 0.18 ratio
        let ratio = dict.compression_ratio("description");
        assert!(ratio < 0.3, "Expected good compression for 'description'");

        // Unknown key should have 1.0 ratio
        let ratio = dict.compression_ratio("unknown_xyz");
        assert!((ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_average_compression_ratio() {
        let dict = AbbrevDict::new();
        let avg = dict.average_compression_ratio();

        // Average should be significantly less than 1.0 (good compression)
        assert!(avg < 0.6, "Expected average compression ratio < 0.6, got {}", avg);
    }
}
