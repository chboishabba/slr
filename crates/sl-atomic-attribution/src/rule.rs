//! Finite registered-rule gate execution over one atomic case registry.
//!
//! This is not the universal legal theorem algebra. It is the runtime consumer
//! that checks whether a source-conditioned rule's already-defined atomic gates
//! are coherent with one retained case registry before a candidate conclusion
//! may be emitted for later governed admission.

use crate::{AtomicCaseRegistry, AtomicGate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredAtomicRuleSchema {
    pub rule_id: String,
    pub source_rule_reference: String,
    pub premise_atoms: Vec<String>,
    pub exception_atoms: Vec<String>,
    pub defeater_atoms: Vec<String>,
    pub jurisdiction_atom: String,
    pub temporal_atom: String,
    pub conclusion_proposition_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleExecutionAuthority {
    RuntimeCandidateOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredAtomicRuleExecutionReceipt {
    pub case_context: String,
    pub rule_id: String,
    pub conclusion_proposition_ref: String,
    pub source_rule_reference: String,
    pub execution_authority: RuleExecutionAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisteredRuleBlock {
    MissingAtom { atom_id: String },
    PremiseGateMismatch { atom_id: String, observed: AtomicGate },
    ExceptionGateMismatch { atom_id: String, observed: AtomicGate },
    DefeaterGateMismatch { atom_id: String, observed: AtomicGate },
    JurisdictionGateMismatch { atom_id: String, observed: AtomicGate },
    TemporalGateMismatch { atom_id: String, observed: AtomicGate },
    MissingRuleId,
    MissingSourceRuleReference,
    MissingConclusionReference,
}

fn gate_for(
    registry: &AtomicCaseRegistry,
    case_context: &str,
    atom_id: &str,
) -> Result<AtomicGate, RegisteredRuleBlock> {
    registry
        .get(case_context, atom_id)
        .map(|entry| entry.gate)
        .ok_or_else(|| RegisteredRuleBlock::MissingAtom {
            atom_id: atom_id.to_owned(),
        })
}

pub fn execute_registered_atomic_rule(
    registry: &AtomicCaseRegistry,
    case_context: &str,
    schema: &RegisteredAtomicRuleSchema,
) -> Result<RegisteredAtomicRuleExecutionReceipt, RegisteredRuleBlock> {
    if schema.rule_id.trim().is_empty() {
        return Err(RegisteredRuleBlock::MissingRuleId);
    }
    if schema.source_rule_reference.trim().is_empty() {
        return Err(RegisteredRuleBlock::MissingSourceRuleReference);
    }
    if schema.conclusion_proposition_ref.trim().is_empty() {
        return Err(RegisteredRuleBlock::MissingConclusionReference);
    }

    // Positive premises are checked first. This ordering is intentional: a
    // known failed premise blocks the consumer before unresolved downstream
    // jurisdiction/temporal coordinates need acquisition for this consumer.
    for atom_id in &schema.premise_atoms {
        let observed = gate_for(registry, case_context, atom_id)?;
        if observed != AtomicGate::FitsThisAtom {
            return Err(RegisteredRuleBlock::PremiseGateMismatch {
                atom_id: atom_id.clone(),
                observed,
            });
        }
    }
    for atom_id in &schema.exception_atoms {
        let observed = gate_for(registry, case_context, atom_id)?;
        if observed != AtomicGate::FailsThisAtom {
            return Err(RegisteredRuleBlock::ExceptionGateMismatch {
                atom_id: atom_id.clone(),
                observed,
            });
        }
    }
    for atom_id in &schema.defeater_atoms {
        let observed = gate_for(registry, case_context, atom_id)?;
        if observed != AtomicGate::FailsThisAtom {
            return Err(RegisteredRuleBlock::DefeaterGateMismatch {
                atom_id: atom_id.clone(),
                observed,
            });
        }
    }

    let jurisdiction = gate_for(registry, case_context, &schema.jurisdiction_atom)?;
    if jurisdiction != AtomicGate::FitsThisAtom {
        return Err(RegisteredRuleBlock::JurisdictionGateMismatch {
            atom_id: schema.jurisdiction_atom.clone(),
            observed: jurisdiction,
        });
    }
    let temporal = gate_for(registry, case_context, &schema.temporal_atom)?;
    if temporal != AtomicGate::FitsThisAtom {
        return Err(RegisteredRuleBlock::TemporalGateMismatch {
            atom_id: schema.temporal_atom.clone(),
            observed: temporal,
        });
    }

    Ok(RegisteredAtomicRuleExecutionReceipt {
        case_context: case_context.to_owned(),
        rule_id: schema.rule_id.clone(),
        conclusion_proposition_ref: schema.conclusion_proposition_ref.clone(),
        source_rule_reference: schema.source_rule_reference.clone(),
        execution_authority: RuleExecutionAuthority::RuntimeCandidateOnly,
    })
}

pub fn cullen_s5b_threshold_schema() -> RegisteredAtomicRuleSchema {
    RegisteredAtomicRuleSchema {
        rule_id: "rule:NSW:CLA:s5B:threshold-open:repository-reconstruction".into(),
        source_rule_reference: "SensibLawNSWCivilLiabilityActAtomicSourceAtlasExact:s5BThresholdRule".into(),
        premise_atoms: vec![
            "atom:NSW:CLA:s5B1a:risk-foreseeable".into(),
            "atom:NSW:CLA:s5B1b:risk-not-insignificant".into(),
            "atom:NSW:CLA:s5B1c:reasonable-person-would-take-proposed-precautions".into(),
        ],
        exception_atoms: Vec::new(),
        defeater_atoms: Vec::new(),
        // These are deliberately required even though the current bounded
        // Cullen registry does not source-pay them. The known third-premise
        // failure blocks this consumer before they become live acquisition gaps.
        jurisdiction_atom: "atom:NSW:CLA:jurisdiction-predicate".into(),
        temporal_atom: "atom:NSW:CLA:effective-at-Cullen-event".into(),
        conclusion_proposition_ref: "prop:NSW:CLA:s5B:threshold-open".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cullen_gold_registry, CULLEN_CONTEXT};

    #[test]
    fn cullen_s5b_rule_is_blocked_by_registered_third_premise_before_downstream_gaps() {
        let registry = cullen_gold_registry();
        let result = execute_registered_atomic_rule(
            &registry,
            CULLEN_CONTEXT,
            &cullen_s5b_threshold_schema(),
        );
        assert_eq!(
            result,
            Err(RegisteredRuleBlock::PremiseGateMismatch {
                atom_id: "atom:NSW:CLA:s5B1c:reasonable-person-would-take-proposed-precautions"
                    .into(),
                observed: AtomicGate::FailsThisAtom,
            })
        );
    }

    #[test]
    fn missing_jurisdiction_is_not_misreported_when_known_premise_already_blocks() {
        let registry = cullen_gold_registry();
        let result = execute_registered_atomic_rule(
            &registry,
            CULLEN_CONTEXT,
            &cullen_s5b_threshold_schema(),
        );
        assert!(!matches!(result, Err(RegisteredRuleBlock::MissingAtom { .. })));
    }
}
