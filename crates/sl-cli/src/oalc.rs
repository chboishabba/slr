use sensiblaw_governed_legal_provider::{
    run_pinned_oalc_stream, OalcCitationMatch, PinnedOalcStreamRequest,
};

fn value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

pub fn run(args: Vec<String>) -> Result<(), String> {
    if args.first().map(String::as_str) != Some("stream-pinned") {
        return Err(
            "usage: sensiblaw legal-follow oalc stream-pinned --revision SHA --citation TEXT [--citation-match exact|contains] [--document-type TYPE] [--source SOURCE] [--jurisdiction JURISDICTION]"
                .into(),
        );
    }
    let revision = value(&args, "--revision")
        .ok_or_else(|| "--revision is required".to_string())?;
    let citation = value(&args, "--citation")
        .ok_or_else(|| "--citation is required".to_string())?;
    let citation_match = match value(&args, "--citation-match").as_deref().unwrap_or("exact") {
        "exact" => OalcCitationMatch::Exact,
        "contains" => OalcCitationMatch::Contains,
        other => return Err(format!("unsupported --citation-match {other:?}")),
    };
    let document_type =
        value(&args, "--document-type").unwrap_or_else(|| "primary_legislation".into());
    let source = value(&args, "--source").filter(|value| !value.is_empty());
    let jurisdiction = value(&args, "--jurisdiction").filter(|value| !value.is_empty());

    let row = run_pinned_oalc_stream(&PinnedOalcStreamRequest {
        revision,
        citation,
        citation_match,
        document_type,
        source,
        jurisdiction,
    })
    .map_err(|error| format!("pinned OALC stream failed: {error:?}"))?;

    let json = serde_json::to_string(&row)
        .map_err(|error| format!("encode pinned OALC row: {error}"))?;
    println!("{json}");
    Ok(())
}
