//! Sprint 8 generic matter runtime.
//!
//! This module composes the already-paid matter workbench, universal
//! explanation index, and projection fabric behind one UI-independent command
//! reducer.  It deliberately does not create semantic/evidential authority.
//! Dioxus, wgpu, keyboard, voice, and other frontends may decode native events
//! into the same MatterCommand values.

use std::collections::BTreeSet;

use sensiblaw_reader_model::ReaderIntent;

use crate::{
    compile_explanation_index_from_state, compile_projection,
    ExplanationIndex, LegalCampaignState, LegalProjectionState, MatterIssueWorkbench,
    LegalWorldCoordinate, ProjectionContext, ProjectionGraph, ProjectionKind, ProjectionQuery,
    ProvenanceAddress, WrongTypeIssueState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatterCommand {
    SelectObject(String),
    FollowTarget(String),
    FocusProvenance(String),
    OpenSource(String),
    Explain(String),
    ExpandExplanation(String),
    SetProjection(ProjectionKind),
    SetRange {
        as_at: Option<String>,
    },
    SetJurisdiction {
        jurisdictions: BTreeSet<String>,
    },
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterInteractionState {
    pub selected_ref: Option<String>,
    pub followed_ref: Option<String>,
    pub provenance_focus_ref: Option<String>,
    pub projection_query: ProjectionQuery,
    pub navigation_stack: Vec<String>,
}

impl MatterInteractionState {
    fn new() -> Self {
        Self {
            selected_ref: None,
            followed_ref: None,
            provenance_focus_ref: None,
            projection_query: ProjectionQuery::new(ProjectionKind::IssueProof),
            navigation_stack: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatterRuntimeEffect {
    None,
    SelectionChanged {
        semantic_ref: String,
    },
    Followed {
        semantic_ref: String,
    },
    ProvenanceFocused {
        semantic_ref: String,
        addresses: Vec<ProvenanceAddress>,
    },
    SourceOpened {
        semantic_ref: String,
        addresses: Vec<ProvenanceAddress>,
    },
    Explained {
        semantic_ref: String,
        dependencies: Vec<String>,
        material_consequences: Vec<String>,
    },
    ProjectionChanged {
        graph: ProjectionGraph,
    },
    Back {
        selected_ref: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterRuntimeReceipt {
    pub effect: MatterRuntimeEffect,
    pub projection: ProjectionGraph,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterRuntime {
    pub workbench: MatterIssueWorkbench,
    pub explanation: ExplanationIndex,
    pub projection_context: ProjectionContext,
    pub interaction: MatterInteractionState,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBoundMatterRuntime {
    pub world: LegalWorldCoordinate,
    pub runtime: MatterRuntime,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl WorldBoundMatterRuntime {
    pub fn new(
        mut runtime: MatterRuntime,
        world: LegalWorldCoordinate,
    ) -> Result<Self, String> {
        runtime.validate()?;
        world.validate()?;

        runtime.interaction.projection_query.as_at = Some(world.as_at.clone());
        runtime.interaction.projection_query.jurisdiction_slice =
            BTreeSet::from([world.jurisdiction_ref.clone()]);

        let bound = Self {
            world,
            runtime,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        bound.validate()?;
        Ok(bound)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.world.validate()?;
        self.runtime.validate()?;
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("WorldBoundMatterRuntime crossed non-promotion boundary".into());
        }
        if self.runtime.interaction.projection_query.as_at.as_deref()
            != Some(self.world.as_at.as_str())
            || self.runtime.interaction.projection_query.jurisdiction_slice
                != BTreeSet::from([self.world.jurisdiction_ref.clone()])
        {
            return Err("MatterRuntime projection coordinates drifted from bound legal world".into());
        }
        Ok(())
    }

    pub fn current_projection(&self) -> Result<ProjectionGraph, String> {
        self.validate()?;
        self.runtime.current_projection()
    }

    /// Dispatch ordinary semantic/read commands while preserving the bound
    /// world.  SetRange/SetJurisdiction are rejected here; callers must create a
    /// distinct LegalWorldCoordinate and rebind explicitly.
    pub fn dispatch(
        &mut self,
        command: MatterCommand,
    ) -> Result<MatterRuntimeReceipt, String> {
        if matches!(
            &command,
            MatterCommand::SetRange { .. } | MatterCommand::SetJurisdiction { .. }
        ) {
            return Err(
                "world-bound runtime requires explicit rebind for time/jurisdiction change"
                    .into(),
            );
        }
        let receipt = self.runtime.dispatch(command)?;
        self.validate()?;
        Ok(receipt)
    }
}

pub fn compile_matter_runtime_from_state(
    workbench: MatterIssueWorkbench,
    issue: &WrongTypeIssueState,
    state: &LegalProjectionState,
    projection_context: ProjectionContext,
) -> Result<MatterRuntime, String> {
    workbench.validate_projection_boundary()?;
    let explanation = compile_explanation_index_from_state(&workbench, issue, state)?;
    explanation.validate()?;

    let runtime = MatterRuntime {
        workbench,
        explanation,
        projection_context,
        interaction: MatterInteractionState::new(),
        candidate_only: true,
        creates_semantic_authority: false,
    };
    runtime.validate()?;
    Ok(runtime)
}

pub fn compile_matter_runtime(
    workbench: MatterIssueWorkbench,
    issue: &WrongTypeIssueState,
    campaign: &LegalCampaignState,
    projection_context: ProjectionContext,
) -> Result<MatterRuntime, String> {
    let state = LegalProjectionState::from(campaign);
    compile_matter_runtime_from_state(workbench, issue, &state, projection_context)
}

pub fn lower_reader_intent(
    intent: ReaderIntent,
    selected_ref: Option<&str>,
) -> Result<MatterCommand, String> {
    let selected = || {
        selected_ref
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| "reader intent requires a selected semantic object".to_owned())
    };

    match intent {
        ReaderIntent::Explain | ReaderIntent::WhyClaim => Ok(MatterCommand::Explain(selected()?)),
        ReaderIntent::OpenSource => Ok(MatterCommand::OpenSource(selected()?)),
        ReaderIntent::ExpandProofCone => Ok(MatterCommand::ExpandExplanation(selected()?)),
        ReaderIntent::Back => Ok(MatterCommand::Back),
    }
}

impl MatterRuntime {
    pub fn validate(&self) -> Result<(), String> {
        if !self.candidate_only || self.creates_semantic_authority {
            return Err("MatterRuntime crossed semantic authority boundary".into());
        }
        self.workbench.validate_projection_boundary()?;
        self.explanation.validate()?;
        Ok(())
    }

    pub fn current_projection(&self) -> Result<ProjectionGraph, String> {
        compile_projection(
            &self.workbench,
            &self.explanation,
            &self.interaction.projection_query,
            &self.projection_context,
        )
    }

    fn require_semantic_ref(&self, semantic_ref: &str) -> Result<(), String> {
        if semantic_ref.trim().is_empty() {
            return Err("MatterCommand carried an empty semantic ref".into());
        }
        if !self.explanation.records.contains_key(semantic_ref) {
            return Err(format!("MatterCommand referenced unknown semantic object {semantic_ref}"));
        }
        Ok(())
    }

    fn push_selection_history(&mut self) {
        if let Some(current) = self.interaction.selected_ref.clone() {
            if self.interaction.navigation_stack.last() != Some(&current) {
                self.interaction.navigation_stack.push(current);
            }
        }
    }

    fn select_for_projection(&mut self, semantic_ref: &str) {
        self.interaction.projection_query.semantic_selection.clear();
        self.interaction
            .projection_query
            .semantic_selection
            .insert(semantic_ref.to_owned());
    }

    pub fn dispatch(&mut self, command: MatterCommand) -> Result<MatterRuntimeReceipt, String> {
        self.validate()?;

        let effect = match command {
            MatterCommand::SelectObject(semantic_ref) => {
                self.require_semantic_ref(&semantic_ref)?;
                self.push_selection_history();
                self.interaction.selected_ref = Some(semantic_ref.clone());
                self.select_for_projection(&semantic_ref);
                MatterRuntimeEffect::SelectionChanged { semantic_ref }
            }
            MatterCommand::FollowTarget(semantic_ref) => {
                self.require_semantic_ref(&semantic_ref)?;
                self.push_selection_history();
                self.interaction.selected_ref = Some(semantic_ref.clone());
                self.interaction.followed_ref = Some(semantic_ref.clone());
                self.select_for_projection(&semantic_ref);
                MatterRuntimeEffect::Followed { semantic_ref }
            }
            MatterCommand::FocusProvenance(semantic_ref) => {
                self.require_semantic_ref(&semantic_ref)?;
                let addresses = self
                    .explanation
                    .get(&semantic_ref)
                    .expect("semantic ref checked above")
                    .provenance
                    .clone();
                self.interaction.provenance_focus_ref = Some(semantic_ref.clone());
                MatterRuntimeEffect::ProvenanceFocused {
                    semantic_ref,
                    addresses,
                }
            }
            MatterCommand::OpenSource(semantic_ref) => {
                self.require_semantic_ref(&semantic_ref)?;
                let addresses = self
                    .explanation
                    .get(&semantic_ref)
                    .expect("semantic ref checked above")
                    .provenance
                    .clone();
                if addresses.is_empty() {
                    return Err(format!("{semantic_ref} has no source address"));
                }
                MatterRuntimeEffect::SourceOpened {
                    semantic_ref,
                    addresses,
                }
            }
            MatterCommand::Explain(semantic_ref)
            | MatterCommand::ExpandExplanation(semantic_ref) => {
                self.require_semantic_ref(&semantic_ref)?;
                let record = self
                    .explanation
                    .get(&semantic_ref)
                    .expect("semantic ref checked above");
                let mut dependencies = record.dependencies.clone();
                dependencies.sort();
                let material_consequences = self
                    .explanation
                    .material_consequences(&semantic_ref)
                    .into_iter()
                    .collect();
                self.push_selection_history();
                self.interaction.selected_ref = Some(semantic_ref.clone());
                self.select_for_projection(&semantic_ref);
                MatterRuntimeEffect::Explained {
                    semantic_ref,
                    dependencies,
                    material_consequences,
                }
            }
            MatterCommand::SetProjection(kind) => {
                self.interaction.projection_query.kind = kind;
                let graph = self.current_projection()?;
                MatterRuntimeEffect::ProjectionChanged { graph }
            }
            MatterCommand::SetRange { as_at } => {
                self.interaction.projection_query.as_at = as_at;
                let graph = self.current_projection()?;
                MatterRuntimeEffect::ProjectionChanged { graph }
            }
            MatterCommand::SetJurisdiction { jurisdictions } => {
                self.interaction.projection_query.jurisdiction_slice = jurisdictions;
                let graph = self.current_projection()?;
                MatterRuntimeEffect::ProjectionChanged { graph }
            }
            MatterCommand::Back => {
                self.interaction.selected_ref = self.interaction.navigation_stack.pop();
                if let Some(selected) = self.interaction.selected_ref.clone() {
                    self.select_for_projection(&selected);
                } else {
                    self.interaction.projection_query.semantic_selection.clear();
                }
                MatterRuntimeEffect::Back {
                    selected_ref: self.interaction.selected_ref.clone(),
                }
            }
        };

        let projection = self.current_projection()?;
        if projection.creates_semantic_authority {
            return Err("MatterRuntime projection unexpectedly created authority".into());
        }

        Ok(MatterRuntimeReceipt {
            effect,
            projection,
            candidate_only: true,
            creates_semantic_authority: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build_australian_calibration_capstone, project_matter_issue_workbench,
        AustralianCalibrationKind, MatterWorkbenchSeed,
    };

    fn runtime(kind: AustralianCalibrationKind) -> MatterRuntime {
        let capstone = build_australian_calibration_capstone(kind).unwrap();
        let campaign = capstone.campaign.hops.last().unwrap();
        let workbench = project_matter_issue_workbench(
            format!("matter:s8:{kind:?}"),
            &capstone.issue,
            campaign,
            MatterWorkbenchSeed::default(),
        )
        .unwrap();
        compile_matter_runtime(
            workbench,
            &capstone.issue,
            campaign,
            ProjectionContext::default(),
        )
        .unwrap()
    }

    #[test]
    fn one_runtime_accepts_every_existing_calibration_without_case_specific_reducer_code() {
        for kind in [
            AustralianCalibrationKind::Mabo,
            AustralianCalibrationKind::Pabai,
            AustralianCalibrationKind::CullenNswCla,
            AustralianCalibrationKind::Glj,
        ] {
            let mut runtime = runtime(kind);
            let anchor = runtime.workbench.observations[0].observation_ref.clone();
            let receipt = runtime
                .dispatch(MatterCommand::SelectObject(anchor.clone()))
                .unwrap();
            assert!(receipt.projection.contains(&anchor));
            assert!(!receipt.creates_semantic_authority);
        }
    }

    #[test]
    fn reader_intents_lower_to_the_shared_matter_command_surface() {
        assert_eq!(
            lower_reader_intent(ReaderIntent::OpenSource, Some("observation:x")).unwrap(),
            MatterCommand::OpenSource("observation:x".into())
        );
        assert_eq!(
            lower_reader_intent(ReaderIntent::WhyClaim, Some("observation:x")).unwrap(),
            MatterCommand::Explain("observation:x".into())
        );
        assert_eq!(
            lower_reader_intent(ReaderIntent::ExpandProofCone, Some("observation:x")).unwrap(),
            MatterCommand::ExpandExplanation("observation:x".into())
        );
        assert_eq!(
            lower_reader_intent(ReaderIntent::Back, None).unwrap(),
            MatterCommand::Back
        );
    }

    #[test]
    fn open_source_and_explain_reuse_s6_provenance_and_reverse_impact() {
        let mut runtime = runtime(AustralianCalibrationKind::Mabo);
        let anchor = runtime.workbench.observations[0].observation_ref.clone();

        let source = runtime
            .dispatch(MatterCommand::OpenSource(anchor.clone()))
            .unwrap();
        match source.effect {
            MatterRuntimeEffect::SourceOpened { addresses, .. } => {
                assert!(!addresses.is_empty());
                assert!(!addresses[0].source_revision_ref.is_empty());
            }
            other => panic!("unexpected effect: {other:?}"),
        }

        let explanation = runtime.dispatch(MatterCommand::Explain(anchor)).unwrap();
        assert!(matches!(
            explanation.effect,
            MatterRuntimeEffect::Explained { .. }
        ));
        assert!(!explanation.creates_semantic_authority);
    }

    #[test]
    fn projection_commands_recompile_s7_without_mutating_semantic_identity() {
        let mut runtime = runtime(AustralianCalibrationKind::CullenNswCla);
        let anchor = runtime.workbench.observations[0].observation_ref.clone();
        runtime
            .dispatch(MatterCommand::SelectObject(anchor.clone()))
            .unwrap();

        let graph = runtime
            .dispatch(MatterCommand::SetProjection(ProjectionKind::Timeline))
            .unwrap()
            .projection;

        assert_eq!(graph.kind, ProjectionKind::Timeline);
        assert!(graph.contains(&anchor));
        assert!(runtime.explanation.get(&anchor).is_some());
        assert!(!graph.creates_semantic_authority);
    }

    #[test]
    fn world_binding_pays_projection_time_and_jurisdiction_without_silent_mutation() {
        use std::collections::BTreeMap;

        let runtime = runtime(AustralianCalibrationKind::Mabo);
        let world = LegalWorldCoordinate {
            world_ref: "world:mabo:qld:2026-09-21".into(),
            matter_ref: "matter:s8:Mabo".into(),
            jurisdiction_ref: "AU-QLD".into(),
            as_at: "2026-09-21".into(),
            source_revisions: BTreeMap::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let mut bound = WorldBoundMatterRuntime::new(runtime, world).unwrap();
        assert_eq!(
            bound.runtime.interaction.projection_query.as_at.as_deref(),
            Some("2026-09-21")
        );
        assert_eq!(
            bound.runtime.interaction.projection_query.jurisdiction_slice,
            BTreeSet::from(["AU-QLD".into()])
        );
        assert!(bound
            .dispatch(MatterCommand::SetRange {
                as_at: Some("2025-01-01".into())
            })
            .is_err());
        assert!(bound
            .dispatch(MatterCommand::SetJurisdiction {
                jurisdictions: BTreeSet::from(["AU-NSW".into()])
            })
            .is_err());
        assert!(!bound.creates_semantic_authority);
        assert!(!bound.creates_claim_truth);
    }

}