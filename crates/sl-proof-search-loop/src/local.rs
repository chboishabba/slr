use crate::query::QueryExpr;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDocument {
    pub document_ref: String,
    pub source_revision_ref: String,
    pub jurisdiction_ref: Option<String>,
    pub court_ref: Option<String>,
    pub date_ref: Option<String>,
    pub canonical_text: String,
    pub citation_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePassage {
    pub document_ref: String,
    pub source_revision_ref: String,
    pub matched_expression_ref: String,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LocalIndex {
    docs: BTreeMap<String, LocalDocument>,
}

impl LocalIndex {
    pub fn insert(&mut self, doc: LocalDocument) {
        self.docs.insert(doc.document_ref.clone(), doc);
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    pub fn search(&self, expr: &QueryExpr) -> Vec<CandidatePassage> {
        self.docs
            .values()
            .filter(|doc| matches_expr(doc, expr))
            .map(|doc| CandidatePassage {
                document_ref: doc.document_ref.clone(),
                source_revision_ref: doc.source_revision_ref.clone(),
                matched_expression_ref: format!("local-query:{expr:?}"),
                candidate_only: true,
            })
            .collect()
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn positions(tokens: &[String], needle: &str) -> Vec<usize> {
    let n = needle.to_lowercase();
    tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| (t == &n).then_some(i))
        .collect()
}

fn phrase_tokens(s: &str) -> Vec<String> {
    tokenize(s)
}

fn phrase_positions(tokens: &[String], phrase: &str) -> Vec<usize> {
    let p = phrase_tokens(phrase);
    if p.is_empty() || p.len() > tokens.len() {
        return Vec::new();
    }
    tokens
        .windows(p.len())
        .enumerate()
        .filter_map(|(i, w)| (w == p.as_slice()).then_some(i))
        .collect()
}

fn proximity(doc: &LocalDocument, left: &QueryExpr, right: &QueryExpr, words: u32, ordered: bool) -> bool {
    let tokens = tokenize(&doc.canonical_text);
    let locs = |q: &QueryExpr| -> Vec<usize> {
        match q {
            QueryExpr::Term(s) => positions(&tokens, s),
            QueryExpr::Phrase(s) => phrase_positions(&tokens, s),
            _ => Vec::new(),
        }
    };
    let l = locs(left);
    let r = locs(right);
    l.iter().any(|a| {
        r.iter().any(|b| {
            if ordered {
                *a <= *b && (*b - *a) <= words as usize
            } else {
                a.abs_diff(*b) <= words as usize
            }
        })
    })
}

fn matches_expr(doc: &LocalDocument, expr: &QueryExpr) -> bool {
    let lower = doc.canonical_text.to_lowercase();
    match expr {
        QueryExpr::Term(s) => tokenize(&lower).iter().any(|t| t == &s.to_lowercase()),
        QueryExpr::Phrase(s) => lower.contains(&s.to_lowercase()),
        QueryExpr::All(xs) => xs.iter().all(|x| matches_expr(doc, x)),
        QueryExpr::Any(xs) => xs.iter().any(|x| matches_expr(doc, x)),
        QueryExpr::Not(x) => !matches_expr(doc, x),
        QueryExpr::Near { left, right, words } => proximity(doc, left, right, *words, false),
        QueryExpr::Precedes { left, right, words } => proximity(doc, left, right, *words, true),
        QueryExpr::Citation(c) => doc.citation_refs.iter().any(|x| x == c),
        QueryExpr::Provision(p) => lower.contains(&p.to_lowercase()),
        QueryExpr::Court(c) => doc.court_ref.as_deref() == Some(c.as_str()),
        QueryExpr::DateRange { start, end } => {
            doc.date_ref.as_ref().is_some_and(|d| d >= start && d <= end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> LocalIndex {
        let mut idx = LocalIndex::default();
        idx.insert(LocalDocument {
            document_ref: "doc:cullen".into(),
            source_revision_ref: "source:cullen:rev:1".into(),
            jurisdiction_ref: Some("AU".into()),
            court_ref: Some("HCA".into()),
            date_ref: Some("2015-01-01".into()),
            canonical_text: "The positive operational act created a foreseeable risk of harm.".into(),
            citation_refs: vec!["Donoghue v Stevenson".into()],
        });
        idx
    }

    #[test]
    fn local_proximity_and_citation_search_are_network_free_by_construction() {
        let q = QueryExpr::Near {
            left: Box::new(QueryExpr::Term("operational".into())),
            right: Box::new(QueryExpr::Term("risk".into())),
            words: 5,
        };
        assert_eq!(index().search(&q).len(), 1);
        assert_eq!(
            index()
                .search(&QueryExpr::Citation("Donoghue v Stevenson".into()))
                .len(),
            1
        );
    }
}
