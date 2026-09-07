#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryExpr {
    Term(String),
    Phrase(String),
    All(Vec<QueryExpr>),
    Any(Vec<QueryExpr>),
    Not(Box<QueryExpr>),
    Near { left: Box<QueryExpr>, right: Box<QueryExpr>, words: u32 },
    Precedes { left: Box<QueryExpr>, right: Box<QueryExpr>, words: u32 },
    Citation(String),
    Provision(String),
    Court(String),
    DateRange { start: String, end: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderQuery {
    pub provider_ref: &'static str,
    pub query_string: String,
    pub network_requests: u64,
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\\\""))
}

pub fn compile_austlii(expr: &QueryExpr) -> ProviderQuery {
    fn go(expr: &QueryExpr) -> String {
        match expr {
            QueryExpr::Term(s) => s.clone(),
            QueryExpr::Phrase(s) => quote(s),
            QueryExpr::All(xs) => xs.iter().map(go).collect::<Vec<_>>().join(" AND "),
            QueryExpr::Any(xs) => format!("({})", xs.iter().map(go).collect::<Vec<_>>().join(" OR ")),
            QueryExpr::Not(x) => format!("NOT ({})", go(x)),
            QueryExpr::Near { left, right, words } => {
                format!("{} w/{} {}", go(left), words, go(right))
            }
            QueryExpr::Precedes { left, right, words } => {
                format!("{} pre/{} {}", go(left), words, go(right))
            }
            QueryExpr::Citation(s) => quote(s),
            QueryExpr::Provision(s) => quote(s),
            QueryExpr::Court(s) => format!("court:{}", quote(s)),
            QueryExpr::DateRange { start, end } => format!("date:[{start} TO {end}]"),
        }
    }
    ProviderQuery { provider_ref: "austlii", query_string: go(expr), network_requests: 0 }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JadePlan {
    TextSearch(String),
    CitedBy(String),
    CasesCited(String),
    LegislationCited(String),
}

pub fn compile_jade_text(expr: &QueryExpr) -> JadePlan {
    JadePlan::TextSearch(compile_austlii(expr).query_string)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalQueryPlan {
    pub expression: QueryExpr,
    pub query_authority: &'static str,
}

pub fn compile_local(expr: &QueryExpr) -> LocalQueryPlan {
    LocalQueryPlan { expression: expr.clone(), query_authority: "candidate_retrieval_only" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_ordered_and_unordered_proximity_without_io() {
        let q = QueryExpr::All(vec![
            QueryExpr::Near {
                left: Box::new(QueryExpr::Phrase("duty of care".into())),
                right: Box::new(QueryExpr::Phrase("government policy".into())),
                words: 10,
            },
            QueryExpr::Precedes {
                left: Box::new(QueryExpr::Term("foreseeable".into())),
                right: Box::new(QueryExpr::Term("harm".into())),
                words: 5,
            },
        ]);
        let out = compile_austlii(&q);
        assert_eq!(out.network_requests, 0);
        assert!(out.query_string.contains("w/10"));
        assert!(out.query_string.contains("pre/5"));
    }
}
