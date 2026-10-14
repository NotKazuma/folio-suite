//! A GitHub release, reduced to what the launcher uses. Everything from the network is hostile:
//! sizes are capped, missing fields become empty values, nothing is indexed.

use serde::Deserialize;

/// Largest release JSON we will parse.
pub const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub notes: String,
    pub page_url: String,
    pub assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct RawAsset {
    #[serde(default)]
    name: String,
    #[serde(default)]
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct RawRelease {
    #[serde(default)]
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    assets: Vec<RawAsset>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    TooLarge,
    Malformed,
}

impl Release {
    pub fn parse(json: &str) -> Result<Release, ParseError> {
        if json.len() > MAX_JSON_BYTES {
            return Err(ParseError::TooLarge);
        }
        let raw: RawRelease = serde_json::from_str(json).map_err(|_| ParseError::Malformed)?;
        if raw.tag_name.is_empty() {
            return Err(ParseError::Malformed);
        }
        Ok(Release {
            tag: raw.tag_name,
            notes: raw.body.unwrap_or_default().chars().take(4000).collect(),
            page_url: raw.html_url,
            assets: raw
                .assets
                .into_iter()
                .filter(|a| !a.name.is_empty() && !a.browser_download_url.is_empty())
                .map(|a| Asset { name: a.name, url: a.browser_download_url, size: a.size })
                .collect(),
        })
    }

    /// The checksum file published with the release (`SHA256SUMS.txt`), if any.
    pub fn checksum_asset(&self) -> Option<&Asset> {
        self.assets.iter().find(|a| {
            let n = a.name.to_lowercase();
            n == "sha256sums" || n == "sha256sums.txt"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_release() {
        let r = Release::parse(
            r#"{"tag_name":"v1.2.0","body":"Fixes","html_url":"https://github.com/NotKazuma/wren-pinion/releases/tag/v1.2.0",
            "assets":[{"name":"a.msi","browser_download_url":"https://github.com/x/a.msi","size":42},
                      {"name":"SHA256SUMS.txt","browser_download_url":"https://github.com/x/s","size":3},
                      {"name":"","browser_download_url":"https://github.com/x/none"}]}"#,
        )
        .unwrap();
        assert_eq!(r.tag, "v1.2.0");
        assert_eq!(r.assets.len(), 2);
        assert_eq!(r.checksum_asset().unwrap().name, "SHA256SUMS.txt");
    }

    #[test]
    fn hostile_input_is_an_error_not_a_panic() {
        assert_eq!(Release::parse("not json"), Err(ParseError::Malformed));
        assert_eq!(Release::parse("{}"), Err(ParseError::Malformed));
        assert_eq!(Release::parse(&" ".repeat(MAX_JSON_BYTES + 1)), Err(ParseError::TooLarge));
        assert!(Release::parse(r#"{"tag_name":"v1","assets":[1,2]}"#).is_err());
    }
}
