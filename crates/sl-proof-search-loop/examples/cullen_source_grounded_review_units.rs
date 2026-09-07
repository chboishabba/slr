use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::judgment_candidates::CitationOccurrenceCandidate;
use sensiblaw_proof_search_loop::reasoning::{
    CitationUse, ConditionCoordinate, ConditionKind, ReasoningRole,
};
use sensiblaw_proof_search_loop::residual_review_shortlist::ResidualShortlistedCitation;
use sensiblaw_proof_search_loop::review_unit_review::{
    compile_reviewed_unit_receipt, reviewed_unit_receipts_to_reasoning_delta,
    ReviewedCitationReviewUnitDecision,
};
use sensiblaw_proof_search_loop::review_units::{
    cluster_shortlisted_citations, CitationReviewUnit,
};
use sensiblaw_proof_search_loop::transition::{
    apply_assessments, ResearchTermination, ResidualAssessment, ResidualAssessmentKind,
};

const DOCUMENT: &str = "document:hca:[2026]-HCA-19:docx";
const SOURCE_REVISION: &str =
    "source-revision:sha256:f171fcaa304de4e1a8be9b7e2a200a181025a81456fa89dec18516805cad15b9";
const CANONICAL_TEXT: &str =
    "sha256:7630c53c8b3759b2292afe145ad6745adb352810409f9d43b267b259527f33eb";

fn item(citation: &str, footnote: u64, paragraph: u64, anchor_text: &str, criterion: &str) -> ResidualShortlistedCitation {
    ResidualShortlistedCitation {
        candidate: CitationOccurrenceCandidate {
            document_ref: DOCUMENT.into(),
            source_revision_ref: SOURCE_REVISION.into(),
            canonical_text_sha256: CANONICAL_TEXT.into(),
            paragraph_ordinal: footnote,
            paragraph_locator_ref: format!("{DOCUMENT}#footnote-{footnote}"),
            reported_paragraph_label: Some(format!("footnote:{footnote}")),
            citation_text: citation.into(),
            paragraph_text: citation.into(),
            anchor_paragraph_locator_refs: vec![format!("{DOCUMENT}#paragraph-{paragraph}")],
            anchor_paragraph_texts: vec![anchor_text.into()],
            lexical_treatment_hints: vec![],
            reviewed: false,
            candidate_only: true,
        },
        matched_criterion_refs: vec![criterion.into()],
    }
}

fn unit<'a>(units: &'a [CitationReviewUnit], citation: &str, paragraph: u64) -> &'a CitationReviewUnit {
    let anchor = format!("{DOCUMENT}#paragraph-{paragraph}");
    units
        .iter()
        .find(|unit| {
            unit.citation_text == citation
                && unit.anchor_paragraph_locator_refs == vec![anchor.clone()]
        })
        .unwrap_or_else(|| panic!("missing source-grounded review unit: {citation} @ {paragraph}"))
}

fn decision(
    unit: &CitationReviewUnit,
    anchor: u64,
    citing_proposition_ref: &str,
    cited_document_ref: &str,
    cited_proposition_ref: &str,
    citation_use: CitationUse,
    reasoning_role: ReasoningRole,
    condition_ref: &str,
    lexical_realisation: &str,
) -> ReviewedCitationReviewUnitDecision {
    let selected_anchor = format!("{DOCUMENT}#paragraph-{anchor}");
    let mut evidence_refs = unit.citation_locator_refs.clone();
    evidence_refs.push(selected_anchor.clone());
    ReviewedCitationReviewUnitDecision {
        review_unit_ref: unit.review_unit_ref.clone(),
        source_revision_ref: unit.source_revision_ref.clone(),
        citation_text: unit.citation_text.clone(),
        selected_anchor_paragraph_locator_ref: selected_anchor,
        citing_proposition_ref: citing_proposition_ref.into(),
        cited_document_ref: cited_document_ref.into(),
        cited_proposition_ref: cited_proposition_ref.into(),
        citation_use,
        reasoning_role,
        condition_coordinates: vec![ConditionCoordinate {
            kind: ConditionKind::Legal,
            condition_ref: condition_ref.into(),
        }],
        judge_or_speaker_ref: None,
        court_ref: Some("HCA".into()),
        jurisdiction_ref: Some("AU".into()),
        temporal_ref: Some("2026-06-17".into()),
        outcome_ref: None,
        remedy_ref: None,
        burden_refs: vec![],
        exception_refs: vec![],
        lexical_realisation: lexical_realisation.into(),
        reviewer_ref: "reviewer:source-grounded-cullen-review".into(),
        evidence_refs,
    }
}

fn main() {
    let p89 = "\tIn Robinson v Chief Constable of West Yorkshire Police, the Supreme Court of the United Kingdom recognised such a duty of care owed by members of the police to avoid causing foreseeable physical injury to innocent passers-by or bystanders to a police arrest. Adopting the language of Lord Mance to describe a \"now established area of general police liability\", the State accepted in argument in this appeal that the duty of care owed by members of the police extends to liability for \"positive negligent conduct which foreseeably and directly inflicts physical injury on the public\".The State accordingly accepted that the OSG officers owed a duty of care in the performance of the function of keeping and preserving the peace in the crowd at the march.";
    let p91 = "\tContrary to the State's contentions, the duty of care owed by the OSG officers is not usefully expressed as a duty to avoid the risk of an emotional crowd reaction,or a duty not to provoke negative reactionsin a crowd.These attempted reformulations incorrectly imply that the appellant's case depends upon establishing an affirmative duty to prevent injury resulting from the crowd's reaction to a police operation. As in Robinson, the appellant's complaint is not that the OSG officers failed to protect her against the risk of being injured, but that their actions resulted in her being injured.";
    let p93 = "\tThe State's argument that the duty of care owed by the OSG officers should not extend so as to render them liable for the criminal acts of others seeks to draw too much from the decision of this Court in Modbury Triangle Shopping Centre Pty Ltd v Anzil.Although the distinction may at times be difficult to draw, there is an important difference in tort law between \"careless acts causing personal injury, for which the law generally imposes liability, and careless omissions to prevent acts [by a third party] ... for which the common law generally imposes no liability\".Modbury is an illustration of the latter. The present case is an instance of the former. In a case such as the present, an intervening action of a third party will not negate liability in negligence \"if the intervening action was in the ordinary course of things the very kind of thing likely to happen as a result of the defendant's negligence\".";
    let p148 = "\tThe intervention by the OSG officers was not said in this Court to be pursuant to any statutory power. They were acting with the same liberty, and subject to the same legal constraints, as any member of the general public. And, contrary to the reliance by the State of New South Wales upon Modbury Triangle Shopping Centre Pty Ltd v Anzil, any liability of the OSG officers in this case would be based upon their positive acts in creating risk, not upon any alleged omission to act to prevent actions by third parties. As Lord Reed said of police officers, in the Supreme Court of the United Kingdom in Robinson v Chief Constable of West Yorkshire Police, the duty of care that applies is not merely the duty arising from an assumption of responsibility but is also \"the ordinary common law duty of care to avoid causing reasonably foreseeable injury to persons and reasonably foreseeable damage to property\".";

    let shortlisted = vec![
        item("[2018] AC 736", 43, 89, p89, "criterion:cullen:positive-negligent-conduct"),
        item("[2018] AC 736", 44, 89, p89, "criterion:cullen:positive-negligent-conduct"),
        item("[2018] AC 736", 45, 91, p91, "criterion:cullen:actions-not-failure-to-protect"),
        item("(2000) 205 CLR 254", 46, 93, p93, "criterion:cullen:careless-act-vs-omission"),
        item("(2000) 205 CLR 254", 48, 93, p93, "criterion:cullen:careless-act-vs-omission"),
        item("(2000) 205 CLR 254", 105, 148, p148, "criterion:cullen:positive-act-vs-omission"),
        item("[2018] AC 736", 106, 148, p148, "criterion:cullen:positive-act-vs-omission"),
        item("[2018] AC 736", 107, 148, p148, "criterion:cullen:positive-act-vs-omission"),
    ];
    let units = cluster_shortlisted_citations(&shortlisted);
    assert_eq!(units.len(), 5);

    let r89 = unit(&units, "[2018] AC 736", 89);
    let r91 = unit(&units, "[2018] AC 736", 91);
    let m93 = unit(&units, "(2000) 205 CLR 254", 93);
    let m148 = unit(&units, "(2000) 205 CLR 254", 148);
    let r148 = unit(&units, "[2018] AC 736", 148);

    let receipts = vec![
        compile_reviewed_unit_receipt(
            r89,
            &decision(
                r89,
                89,
                "prop:cullen:joint-positive-negligent-conduct-duty",
                "case:robinson-v-chief-constable-west-yorkshire-police",
                "prop:robinson:cullen-characterisation-police-positive-conduct-duty",
                CitationUse::ReliedOn,
                ReasoningRole::Rule,
                "condition:cullen:police-positive-conduct",
                "recognised police duty for foreseeable physical injury",
            ),
        )
        .unwrap(),
        compile_reviewed_unit_receipt(
            r91,
            &decision(
                r91,
                91,
                "prop:cullen:joint-actions-not-failure-to-protect",
                "case:robinson-v-chief-constable-west-yorkshire-police",
                "prop:robinson:cullen-characterisation-action-not-omission",
                CitationUse::ReliedOn,
                ReasoningRole::Analogy,
                "condition:cullen:action-not-protection-omission",
                "as in Robinson, complaint concerns actions causing injury",
            ),
        )
        .unwrap(),
        compile_reviewed_unit_receipt(
            m93,
            &decision(
                m93,
                93,
                "prop:cullen:joint-modbury-act-omission-distinction",
                "case:modbury-triangle-shopping-centre-v-anzil",
                "prop:modbury:cullen-characterisation-third-party-omission",
                CitationUse::Distinguished,
                ReasoningRole::Distinction,
                "condition:cullen:careless-act-vs-third-party-omission",
                "Modbury is omission-side; Cullen is act-side",
            ),
        )
        .unwrap(),
        compile_reviewed_unit_receipt(
            m148,
            &decision(
                m148,
                148,
                "prop:cullen:edelman-positive-act-imposed-duty",
                "case:modbury-triangle-shopping-centre-v-anzil",
                "prop:modbury:cullen-characterisation-third-party-omission",
                CitationUse::Distinguished,
                ReasoningRole::Distinction,
                "condition:cullen:positive-act-not-third-party-omission",
                "contrary to reliance on Modbury, liability is based on positive acts",
            ),
        )
        .unwrap(),
        compile_reviewed_unit_receipt(
            r148,
            &decision(
                r148,
                148,
                "prop:cullen:edelman-positive-act-imposed-duty",
                "case:robinson-v-chief-constable-west-yorkshire-police",
                "prop:robinson:cullen-characterisation-ordinary-duty-foreseeable-injury",
                CitationUse::ReliedOn,
                ReasoningRole::Rule,
                "condition:cullen:ordinary-duty-foreseeable-injury",
                "ordinary common-law duty to avoid reasonably foreseeable injury",
            ),
        )
        .unwrap(),
    ];

    let delta = reviewed_unit_receipts_to_reasoning_delta(&receipts);
    assert_eq!(delta.edges.len(), 5);
    assert_eq!(delta.delta_authority, "experimental_candidate_only");
    assert!(delta.edges.iter().all(|edge| edge.reviewed && edge.candidate_only));
    assert_eq!(
        delta
            .edges
            .iter()
            .filter(|edge| edge.citation_use == CitationUse::Distinguished)
            .count(),
        2
    );

    let frontier = ProofFrontier {
        consumer_ref: "consumer:cullen-positive-operational-duty".into(),
        frontier_ref: "frontier:cullen:review-units:v0".into(),
        residuals: vec![ProofResidual {
            residual_ref: "residual:cullen-positive-operational-act".into(),
            proposition_ref: "prop:cullen-positive-operational-duty".into(),
            producer_class_ref: "producer:exact-primary-authority".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: Some("official-primary-case".into()),
            salience: 10,
            dependency_refs: vec![SOURCE_REVISION.into()],
            status: ResidualStatus::Open,
        }],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };

    let (next, transition) = apply_assessments(
        &frontier,
        "frontier:cullen:review-units:v1",
        &[ResidualAssessment {
            residual_ref: "residual:cullen-positive-operational-act".into(),
            kind: ResidualAssessmentKind::Narrowed,
            observed_proof_reduction: 1,
            assessment_ref: "assessment:cullen:reviewed-robinson-modbury-treatment".into(),
            assessment_authority: "experimental_candidate_only",
        }],
    )
    .unwrap();

    assert_eq!(next.residuals[0].status, ResidualStatus::Open);
    assert_eq!(transition.termination, ResearchTermination::Continue);
    println!(
        "cullen_source_grounded_review_units=PASS reviewed_units={} reasoning_edges={} distinguished={} frontier_status=Open termination=Continue network=0 authority=experimental_candidate_only",
        receipts.len(),
        delta.edges.len(),
        delta.edges
            .iter()
            .filter(|edge| edge.citation_use == CitationUse::Distinguished)
            .count(),
    );
}
