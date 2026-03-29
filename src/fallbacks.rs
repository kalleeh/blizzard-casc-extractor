//! Embedded fallback data for known SC:R files that may not be present in
//! a local install or CDN fetch.
//!
//! Keys are canonical CASC paths (locale-prefixed, backslash-separated).
//! These files are compiled directly into the binary via `include_bytes!`.
//!
//! The fallback is only consulted after a live extraction attempt fails —
//! CDN / local-install data always takes priority.

/// A single fallback entry: (casc_path, bytes).
type FallbackEntry = (&'static str, &'static [u8]);

/// All embedded fallback files.
static ENTRIES: &[FallbackEntry] = &[
    (
        r"locales\enUS\Assets\rez\statbtnn.ui.json",
        include_bytes!("../fallbacks/ui/statbtnn.ui.json"),
    ),
    (
        r"locales\enUS\Assets\rez\statbtnp.ui.json",
        include_bytes!("../fallbacks/ui/statbtnp.ui.json"),
    ),
    (
        r"locales\enUS\Assets\rez\statbtnt.ui.json",
        include_bytes!("../fallbacks/ui/statbtnt.ui.json"),
    ),
    (
        r"locales\enUS\Assets\rez\statbtnz.ui.json",
        include_bytes!("../fallbacks/ui/statbtnz.ui.json"),
    ),
    (
        r"locales\enUS\Assets\rez\statdata.ui.json",
        include_bytes!("../fallbacks/ui/statdata.ui.json"),
    ),
    (
        r"locales\enUS\Assets\rez\statport.ui.json",
        include_bytes!("../fallbacks/ui/statport.ui.json"),
    ),
];

/// Look up embedded fallback bytes for a CASC path.
///
/// `casc_path` may use either `/` or `\` as the separator; both are handled.
/// The lookup is case-insensitive so that minor casing differences don't matter.
/// Paths that omit the leading `locales\<locale>\` segment are also matched —
/// this handles the case where the local CASC storage returns paths without
/// the locale prefix (e.g. `enUS\Assets\rez\statbtnn.ui.json`).
pub fn get(casc_path: &str) -> Option<&'static [u8]> {
    // Normalise to backslash for comparison.
    let normalised = casc_path.replace('/', "\\");
    let lower = normalised.to_lowercase();

    ENTRIES.iter().find(|(key, _)| {
        let kl = key.to_lowercase();
        // Exact match.
        if kl == lower {
            return true;
        }
        // The stored key has a `locales\<locale>\` prefix but the caller may
        // have omitted it (paths from CascStorage often lack it).
        // Check whether the key ends with `\<lower>`.
        if kl.ends_with(&format!("\\{}", lower)) {
            return true;
        }
        false
    }).map(|(_, data)| *data)
}

/// Return all embedded entries whose key contains `pattern` (case-insensitive)
/// and also contains `locale` (case-insensitive) as a substring.
pub fn search(pattern: &str, locale: &str) -> Vec<&'static str> {
    let pat = pattern.to_lowercase();
    let loc = locale.to_lowercase();
    ENTRIES
        .iter()
        .filter(|(key, _)| {
            let kl = key.to_lowercase();
            kl.contains(&pat) && kl.contains(&loc)
        })
        .map(|(key, _)| *key)
        .collect()
}
