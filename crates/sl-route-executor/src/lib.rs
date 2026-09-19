use std::io::{ErrorKind, Read, Write};

use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use sensiblaw_world_store::{decode_record, WireRecord, WorldRecordKind};
use thiserror::Error;
use url::Url;

pub const SLRX_MAGIC: [u8; 4] = *b"SLRX";
pub const SLRX_VERSION: u16 = 1;
const MAX_TEXT_BYTES: usize = 64 << 20;
const FLAG_CANDIDATE_ONLY: u8 = 1 << 0;
const FLAG_SEMANTIC_PROMOTION: u8 = 1 << 1;

#[derive(Debug, Error)]
pub enum RouteExecutorError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("world wire error: {0}")]
    WorldWire(#[from] sensiblaw_world_store::WorldStoreError),
    #[error("invalid selected route: {0}")]
    InvalidRoute(String),
    #[error("invalid acquired source: {0}")]
    InvalidSource(String),
    #[error("fetch error: {0}")]
    Fetch(String),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquiredSourceKind {
    WikipediaRenderedHtml = 1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquiredSource {
    pub kind: AcquiredSourceKind,
    pub document_ref: String,
    pub source_ref: String,
    pub language: String,
    pub revision_ref: String,
    pub canonical_url: String,
    pub source_sha256: [u8; 32],
    pub text: String,
    pub candidate_only: bool,
    pub semantic_promotion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionReceipt {
    pub selected_routes_seen: u64,
    pub executable_routes_seen: u64,
    pub sources_emitted: u64,
    pub deferred_routes: u64,
    pub acquisition_creates_claim_truth: bool,
    pub candidate_only: bool,
    pub semantic_promotion: bool,
}

#[derive(Debug)]
struct SelectedRoute {
    producer: u8,
    route_family: u8,
    source_ref: String,
    target_ref: String,
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32, RouteExecutorError> {
    let mut b = [0u8; 4];
    reader.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn write_u32<W: Write>(writer: &mut W, value: u32) -> Result<(), RouteExecutorError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn read_text<R: Read>(reader: &mut R) -> Result<String, RouteExecutorError> {
    let len = read_u32(reader)? as usize;
    if len > MAX_TEXT_BYTES {
        return Err(RouteExecutorError::InvalidSource("text field too large".into()));
    }
    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| RouteExecutorError::InvalidSource("text is not UTF-8".into()))
}

fn write_text<W: Write>(writer: &mut W, value: &str) -> Result<(), RouteExecutorError> {
    if value.len() > MAX_TEXT_BYTES {
        return Err(RouteExecutorError::InvalidSource("text field too large".into()));
    }
    write_u32(writer, value.len() as u32)?;
    writer.write_all(value.as_bytes())?;
    Ok(())
}

fn read_exact_or_eof<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<bool, RouteExecutorError> {
    let mut offset = 0;
    while offset < buf.len() {
        match reader.read(&mut buf[offset..]) {
            Ok(0) if offset == 0 => return Ok(false),
            Ok(0) => return Err(RouteExecutorError::InvalidSource("truncated SLRX frame".into())),
            Ok(n) => offset += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(true)
}

fn parse_selected_route(record: &WireRecord) -> Result<Option<SelectedRoute>, RouteExecutorError> {
    if record.kind != WorldRecordKind::RouteAction {
        return Ok(None);
    }
    if record.payload.len() < 8 || &record.payload[..4] != b"RTA2" {
        return Ok(None);
    }
    if record.payload[6] != 1 || record.payload[7] != 0 {
        return Err(RouteExecutorError::InvalidRoute(
            "selected route must remain candidate-only and non-promoting".into(),
        ));
    }
    let producer = record.payload[4];
    let route_family = record.payload[5];
    let mut cursor = std::io::Cursor::new(&record.payload[8..]);
    let _intent_ref = read_text(&mut cursor)?;
    let _candidate_ref = read_text(&mut cursor)?;
    let source_ref = read_text(&mut cursor)?;
    let target_ref = read_text(&mut cursor)?;
    let _property_ref = read_text(&mut cursor)?;
    Ok(Some(SelectedRoute { producer, route_family, source_ref, target_ref }))
}

fn wikipedia_language(url: &Url) -> Option<String> {
    let host = url.host_str()?;
    let suffix = ".wikipedia.org";
    if !host.ends_with(suffix) {
        return None;
    }
    let lang = &host[..host.len() - suffix.len()];
    if lang.is_empty() || lang.contains('.') {
        return None;
    }
    Some(lang.to_owned())
}

fn validate_wikipedia_target(value: &str) -> Result<(Url, String), RouteExecutorError> {
    let url = Url::parse(value).map_err(|e| RouteExecutorError::InvalidRoute(format!("invalid URL: {e}")))?;
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return Err(RouteExecutorError::InvalidRoute("Wikipedia route must use credential-free HTTPS".into()));
    }
    let language = wikipedia_language(&url)
        .ok_or_else(|| RouteExecutorError::InvalidRoute("route target is not a Wikipedia article host".into()))?;
    Ok((url, language))
}

fn rendered_text(html: &[u8]) -> Result<String, RouteExecutorError> {
    let source = std::str::from_utf8(html)
        .map_err(|_| RouteExecutorError::InvalidSource("Wikipedia HTML is not UTF-8".into()))?;
    let document = Html::parse_document(source);
    let preferred = Selector::parse("main p, main h1, main h2, main h3, main li")
        .map_err(|e| RouteExecutorError::InvalidSource(format!("selector error: {e:?}")))?;
    let mut chunks = Vec::new();
    for node in document.select(&preferred) {
        let text = node.text().map(str::trim).filter(|v| !v.is_empty()).collect::<Vec<_>>().join(" ");
        if !text.is_empty() {
            chunks.push(text);
        }
    }
    if chunks.is_empty() {
        let main = Selector::parse("main")
            .map_err(|e| RouteExecutorError::InvalidSource(format!("selector error: {e:?}")))?;
        if let Some(node) = document.select(&main).next() {
            let text = node.text().map(str::trim).filter(|v| !v.is_empty()).collect::<Vec<_>>().join(" ");
            if !text.is_empty() {
                chunks.push(text);
            }
        }
    }
    let text = chunks.join("\n");
    if text.is_empty() {
        return Err(RouteExecutorError::InvalidSource("Wikipedia article yielded no readable main text".into()));
    }
    Ok(text)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn hex_digest(digest: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

pub fn encode_acquired_source<W: Write>(writer: &mut W, source: &AcquiredSource) -> Result<(), RouteExecutorError> {
    writer.write_all(&SLRX_MAGIC)?;
    writer.write_all(&SLRX_VERSION.to_le_bytes())?;
    let mut flags = 0u8;
    if source.candidate_only { flags |= FLAG_CANDIDATE_ONLY; }
    if source.semantic_promotion { flags |= FLAG_SEMANTIC_PROMOTION; }
    writer.write_all(&[source.kind as u8, flags])?;
    write_text(writer, &source.document_ref)?;
    write_text(writer, &source.source_ref)?;
    write_text(writer, &source.language)?;
    write_text(writer, &source.revision_ref)?;
    write_text(writer, &source.canonical_url)?;
    writer.write_all(&source.source_sha256)?;
    write_text(writer, &source.text)?;
    Ok(())
}

pub fn decode_acquired_source<R: Read>(reader: &mut R) -> Result<Option<AcquiredSource>, RouteExecutorError> {
    let mut magic = [0u8; 4];
    if !read_exact_or_eof(reader, &mut magic)? { return Ok(None); }
    if magic != SLRX_MAGIC { return Err(RouteExecutorError::InvalidSource("bad SLRX magic".into())); }
    let mut version = [0u8; 2];
    reader.read_exact(&mut version)?;
    if u16::from_le_bytes(version) != SLRX_VERSION {
        return Err(RouteExecutorError::InvalidSource("unsupported SLRX version".into()));
    }
    let mut tags = [0u8; 2];
    reader.read_exact(&mut tags)?;
    let kind = match tags[0] {
        1 => AcquiredSourceKind::WikipediaRenderedHtml,
        other => return Err(RouteExecutorError::InvalidSource(format!("unknown source kind {other}"))),
    };
    let candidate_only = tags[1] & FLAG_CANDIDATE_ONLY != 0;
    let semantic_promotion = tags[1] & FLAG_SEMANTIC_PROMOTION != 0;
    if !candidate_only || semantic_promotion {
        return Err(RouteExecutorError::InvalidSource("acquired source must remain candidate-only and non-promoting".into()));
    }
    let document_ref = read_text(reader)?;
    let source_ref = read_text(reader)?;
    let language = read_text(reader)?;
    let revision_ref = read_text(reader)?;
    let canonical_url = read_text(reader)?;
    let mut source_sha256 = [0u8; 32];
    reader.read_exact(&mut source_sha256)?;
    let text = read_text(reader)?;
    Ok(Some(AcquiredSource {
        kind, document_ref, source_ref, language, revision_ref, canonical_url,
        source_sha256, text, candidate_only, semantic_promotion,
    }))
}

pub fn execute_selected_routes_with_fetcher<R, W, F>(
    reader: &mut R,
    writer: &mut W,
    mut fetcher: F,
) -> Result<ExecutionReceipt, RouteExecutorError>
where
    R: Read,
    W: Write,
    F: FnMut(&str) -> Result<(Vec<u8>, Option<String>), RouteExecutorError>,
{
    let mut selected_routes_seen = 0u64;
    let mut executable_routes_seen = 0u64;
    let mut sources_emitted = 0u64;
    let mut deferred_routes = 0u64;

    while let Some(record) = decode_record(reader)? {
        let Some(route) = parse_selected_route(&record)? else { continue; };
        selected_routes_seen += 1;
        // Only the currently paid executable seam: ArticleSemantic × WikipediaArticle.
        if route.producer != 1 || route.route_family != 2 {
            deferred_routes += 1;
            continue;
        }
        executable_routes_seen += 1;
        let (url, language) = validate_wikipedia_target(&route.target_ref)?;
        let (html, etag) = fetcher(url.as_str())?;
        let text = rendered_text(&html)?;
        let source_sha256 = sha256(text.as_bytes());
        let revision_ref = etag.unwrap_or_else(|| format!("sha256:{}", hex_digest(&source_sha256)));
        let document_ref = format!("wikipedia:{}:{}:{}", route.source_ref, language, revision_ref);
        let source = AcquiredSource {
            kind: AcquiredSourceKind::WikipediaRenderedHtml,
            document_ref,
            source_ref: route.source_ref,
            language,
            revision_ref,
            canonical_url: url.to_string(),
            source_sha256,
            text,
            candidate_only: true,
            semantic_promotion: false,
        };
        encode_acquired_source(writer, &source)?;
        sources_emitted += 1;
    }

    Ok(ExecutionReceipt {
        selected_routes_seen,
        executable_routes_seen,
        sources_emitted,
        deferred_routes,
        acquisition_creates_claim_truth: false,
        candidate_only: true,
        semantic_promotion: false,
    })
}

pub fn acquire_wikipedia_article(
    source_ref: &str,
    target_url: &str,
) -> Result<AcquiredSource, RouteExecutorError> {
    if source_ref.trim().is_empty() {
        return Err(RouteExecutorError::InvalidRoute(
            "Wikipedia source reference must not be empty".into(),
        ));
    }
    let (url, language) = validate_wikipedia_target(target_url)?;
    let response = ureq::get(url.as_str())
        .set("User-Agent", "SensibLaw-SLR/1.0 (+https://github.com/chboishabba/slr)")
        .call()
        .map_err(|error| RouteExecutorError::Fetch(error.to_string()))?;
    let etag = response.header("etag").map(ToOwned::to_owned);
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;
    let text = rendered_text(&bytes)?;
    let source_sha256 = sha256(text.as_bytes());
    let revision_ref = etag.unwrap_or_else(|| format!("sha256:{}", hex_digest(&source_sha256)));
    Ok(AcquiredSource {
        kind: AcquiredSourceKind::WikipediaRenderedHtml,
        document_ref: format!("wikipedia:{source_ref}:{language}:{revision_ref}"),
        source_ref: source_ref.to_owned(),
        language,
        revision_ref,
        canonical_url: url.to_string(),
        source_sha256,
        text,
        candidate_only: true,
        semantic_promotion: false,
    })
}

pub fn execute_selected_routes<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> Result<ExecutionReceipt, RouteExecutorError> {
    execute_selected_routes_with_fetcher(reader, writer, |url| {
        let response = ureq::get(url)
            .set("User-Agent", "SensibLaw-SLR/1.0 (+https://github.com/chboishabba/slr)")
            .call()
            .map_err(|e| RouteExecutorError::Fetch(e.to_string()))?;
        let etag = response.header("etag").map(ToOwned::to_owned);
        let mut bytes = Vec::new();
        response.into_reader().read_to_end(&mut bytes)?;
        Ok((bytes, etag))
    })
}
