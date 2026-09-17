use std::collections::BTreeMap;

use crate::world_expansion_runner::{
    PreparedWorldExpansionCycle, RecurrentRunBlocker, RecurrentRunBlockerKind,
    WorldExpansionCycleSource,
};
use crate::world_expansion_session::WorldExpansionSession;
use crate::world_identity::WorldObjectIdentity;

#[derive(Debug, Clone)]
pub struct IdentityCoherentCycleSource<S> {
    inner: S,
    committed_representation_classes: BTreeMap<String, String>,
    pending_identity: Option<WorldObjectIdentity>,
    observed_lineage_receipts: usize,
}

impl<S> IdentityCoherentCycleSource<S> {
    #[must_use]
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            committed_representation_classes: BTreeMap::new(),
            pending_identity: None,
            observed_lineage_receipts: 0,
        }
    }

    fn register_committed_pending(
        &mut self,
        session: &WorldExpansionSession,
    ) -> Result<(), RecurrentRunBlocker> {
        if session.lineages.len() < self.observed_lineage_receipts {
            return Err(RecurrentRunBlocker::new(
                RecurrentRunBlockerKind::Other,
                "identity-coherence:session-lineage-regressed",
            ));
        }
        if session.lineages.len() == self.observed_lineage_receipts {
            return Ok(());
        }
        let Some(identity) = self.pending_identity.take() else {
            return Err(RecurrentRunBlocker::new(
                RecurrentRunBlockerKind::Other,
                "identity-coherence:committed-lineage-without-pending-identity",
            ));
        };
        let identity_class_ref = identity.identity_class_ref.clone();
        for representation in identity.representation_refs {
            match self.committed_representation_classes.get(&representation) {
                Some(existing) if existing != &identity_class_ref => {
                    return Err(RecurrentRunBlocker::new(
                        RecurrentRunBlockerKind::IdentityReviewRequired,
                        format!(
                            "identity-coherence:representation:{representation}:existing:{existing}:proposed:{identity_class_ref}"
                        ),
                    ));
                }
                _ => {
                    self.committed_representation_classes
                        .insert(representation, identity_class_ref.clone());
                }
            }
        }
        self.observed_lineage_receipts = session.lineages.len();
        Ok(())
    }

    fn validate_proposed_identity(
        &self,
        identity: &WorldObjectIdentity,
    ) -> Result<(), RecurrentRunBlocker> {
        for representation in &identity.representation_refs {
            if let Some(existing) = self.committed_representation_classes.get(representation) {
                if existing != &identity.identity_class_ref {
                    return Err(RecurrentRunBlocker::new(
                        RecurrentRunBlockerKind::IdentityReviewRequired,
                        format!(
                            "identity-coherence:representation:{representation}:existing:{existing}:proposed:{}",
                            identity.identity_class_ref
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl<S> WorldExpansionCycleSource for IdentityCoherentCycleSource<S>
where
    S: WorldExpansionCycleSource,
{
    fn prepare_next_cycle(
        &mut self,
        session: &WorldExpansionSession,
    ) -> Result<PreparedWorldExpansionCycle, RecurrentRunBlocker> {
        self.register_committed_pending(session)?;
        let prepared = self.inner.prepare_next_cycle(session)?;
        self.validate_proposed_identity(&prepared.identity_resolution.identity)?;
        self.pending_identity = Some(prepared.identity_resolution.identity.clone());
        Ok(prepared)
    }
}
