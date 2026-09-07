#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COUNTERFACTUAL = ROOT / "crates" / "sl-legal-counterfactual" / "src" / "lib.rs"
FOLLOW = ROOT / "crates" / "sl-legal-follow-plan" / "src" / "lib.rs"
WORKSPACE = ROOT / "Cargo.toml"

counterfactual = COUNTERFACTUAL.read_text(encoding="utf-8")
follow = FOLLOW.read_text(encoding="utf-8")
workspace = WORKSPACE.read_text(encoding="utf-8")

required_counterfactual = [
    "pub struct CounterfactualWorld",
    "pub struct CounterfactualFamily",
    "pub enum WorldAdmissibility",
    "pub enum CorrectionKind",
    "EventDeletionOnly",
    "CorrectedInstitutionalRelation",
    "pub enum PhysicalCompatibility",
    "pub enum CausalIdentificationStatus",
    "Underidentified",
    "AlternativeWorldSearchIncomplete",
    "pub enum CounterfactualResidual",
    "pub enum RequiredProducer",
    "pub const fn producer_for",
    "pub struct LegalCausationAssessment",
    "pub enum LegalConsumer",
    "multiple_admissible_worlds_with_different_outcomes_are_underidentified",
    "one_located_world_does_not_prove_unique_counterfactual_if_search_is_open",
    "causal_dependence_does_not_close_liability_or_remedy",
]

required_follow = [
    "pub enum PlanState",
    "ReadyPersisted",
    "BlockedMissingContext",
    "BlockedAcquisitionRequired",
    "pub struct LegalSourceDemand",
    "pub struct PersistedLegalSource",
    "pub struct LegalSourcePlan",
    "pub fn plan_legal_sources",
    "pub fn demand_from_counterfactual_residual",
    'authority: "acquisition_plan_only"',
    "compatible_persisted_legal_source_absent",
    "non_source_residual_does_not_broaden_into_legal_follow",
    "absent_compatible_source_is_work_not_negative_evidence",
    "typed_plan_selects_only_compatible_persisted_revision",
]

missing = [
    *(f"counterfactual:{needle}" for needle in required_counterfactual if needle not in counterfactual),
    *(f"follow:{needle}" for needle in required_follow if needle not in follow),
]
if missing:
    raise SystemExit(f"legal counterfactual contract missing: {missing}")

for crate_name in ["sl-legal-counterfactual", "sl-legal-follow-plan"]:
    if f'"crates/{crate_name}"' not in workspace:
        raise SystemExit(f"workspace missing {crate_name}")

forbidden_counterfactual = [
    "spacy",
    "regex::",
    "Regex::",
    "postgres",
    "sqlx",
    "reqwest",
    "ureq",
    "publish(",
    "auto_admit",
    "automatic_admission",
]
for forbidden in forbidden_counterfactual:
    if forbidden in counterfactual:
        raise SystemExit(f"counterfactual hot-path violation: {forbidden}")

forbidden_follow = [
    "reqwest",
    "ureq",
    "hyper::",
    "TcpStream",
    "bounded_follow",
    "follow_legal_sources",
    "publish(",
    "auto_admit",
]
for forbidden in forbidden_follow:
    if forbidden in follow:
        raise SystemExit(f"legal-follow planner acquired forbidden execution capability: {forbidden}")

# The source-plan compiler is allowed to emit a demand only from producer kinds
# that actually need source/authority work. Scope, liability, remedy, factual
# comparison and physical-model residuals must stay on their native lanes.
if "RequiredProducer::LegalSourceResolver | RequiredProducer::AuthorityResolver" not in follow:
    raise SystemExit("legal-follow broadening gate missing exact source/authority producer restriction")

print("legal counterfactual + source-follow contract PASS")
