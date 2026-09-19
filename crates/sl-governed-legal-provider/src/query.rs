use std::collections::BTreeMap;

pub const AUSTLII_SINO_ENDPOINT: &str = "https://www.austlii.edu.au/cgi-bin/sinosrch.cgi";
pub const AUSTLII_REFERER: &str = "https://www.austlii.edu.au/";
pub const AUSTLII_BROWSER_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
pub const SENSIBLAW_UA: &str = "SensibLaw/0.1 governed-legal-provider";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinoMethod {
    Any,
    Or,
    All,
    Near,
    Phrase,
    Legis,
    Title,
    Boolean,
}

impl SinoMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Or => "or",
            Self::All => "all",
            Self::Near => "near",
            Self::Phrase => "phrase",
            Self::Legis => "legis",
            Self::Title => "title",
            Self::Boolean => "boolean",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinoQuery {
    pub meta: String,
    pub query: String,
    pub method: SinoMethod,
    pub results: u32,
    pub offset: u32,
    pub rank: Option<String>,
    pub callback: Option<String>,
    pub mask_path: Vec<String>,
    pub mask_by_phc: BTreeMap<String, Vec<String>>,
}

pub fn form_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            other => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push('%');
                out.push(HEX[(other >> 4) as usize] as char);
                out.push(HEX[(other & 0x0f) as usize] as char);
            }
        }
    }
    out
}

pub fn build_sino_url(query: &SinoQuery) -> String {
    let mut params = vec![
        ("meta".to_string(), query.meta.clone()),
        ("method".to_string(), query.method.as_str().to_string()),
        ("query".to_string(), query.query.clone()),
        ("results".to_string(), query.results.to_string()),
        ("offset".to_string(), query.offset.to_string()),
    ];
    if let Some(rank) = &query.rank {
        params.push(("rank".into(), rank.clone()));
    }
    if let Some(callback) = &query.callback {
        params.push(("callback".into(), callback.clone()));
    }
    for path in &query.mask_path {
        params.push(("mask_path".into(), path.clone()));
    }
    for (alias, paths) in &query.mask_by_phc {
        for path in paths {
            params.push((format!("mask_{alias}"), path.clone()));
        }
    }
    let query_string = params
        .into_iter()
        .map(|(key, value)| format!("{}={}", form_encode(&key), form_encode(&value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{AUSTLII_SINO_ENDPOINT}?{query_string}")
}

pub fn deterministic_mnc_to_austlii(citation: &str) -> Option<String> {
    let parts: Vec<_> = citation.split_whitespace().collect();
    if parts.len() != 3 {
        return None;
    }
    let year = parts[0].trim_matches(['[', ']']);
    let court = parts[1];
    let number = parts[2];
    if year.len() != 4 || year.parse::<u32>().is_err() || number.parse::<u32>().is_err() {
        return None;
    }
    let jurisdiction = match court {
        "HCA" | "FCA" | "FCAFC" => "cth",
        "QCA" | "QSC" => "qld",
        "NSWCA" | "NSWSC" => "nsw",
        "VSCA" | "VSC" => "vic",
        "WASCA" | "WASC" => "wa",
        "SASCA" | "SASC" => "sa",
        "TASFC" | "TASSC" => "tas",
        "NTCA" | "NTSC" => "nt",
        "ACTCA" | "ACTSC" => "act",
        _ => return None,
    };
    Some(format!(
        "https://www.austlii.edu.au/cgi-bin/viewdoc/au/cases/{jurisdiction}/{court}/{year}/{number}.html"
    ))
}

pub fn jade_exact_mnc_url(citation: &str) -> String {
    format!("https://jade.io/search/{}", form_encode(citation))
}

pub fn jade_search_url(term: &str) -> String {
    format!("https://jade.io/search/{}", form_encode(term))
}

pub fn is_austlii_url(url: &str) -> bool {
    url.starts_with("https://www.austlii.edu.au/") || url.starts_with("https://austlii.edu.au/")
}

pub fn is_jade_url(url: &str) -> bool {
    url.starts_with("https://jade.io/") || url.starts_with("https://www.jade.io/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sino_url_matches_historical_parameter_order() {
        let query = SinoQuery {
            meta: "/au".into(),
            query: "positive operational act".into(),
            method: SinoMethod::Phrase,
            results: 50,
            offset: 0,
            rank: None,
            callback: None,
            mask_path: Vec::new(),
            mask_by_phc: BTreeMap::new(),
        };
        assert_eq!(
            build_sino_url(&query),
            "https://www.austlii.edu.au/cgi-bin/sinosrch.cgi?meta=%2Fau&method=phrase&query=positive+operational+act&results=50&offset=0"
        );
    }

    #[test]
    fn known_hca_mnc_lowers_without_search() {
        assert_eq!(
            deterministic_mnc_to_austlii("[2026] HCA 19").as_deref(),
            Some("https://www.austlii.edu.au/cgi-bin/viewdoc/au/cases/cth/HCA/2026/19.html")
        );
    }
}
