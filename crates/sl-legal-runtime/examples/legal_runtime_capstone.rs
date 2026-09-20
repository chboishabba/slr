use std::fs;
use std::path::PathBuf;

use sensiblaw_legal_runtime::{
    build_australian_calibration_capstone, build_m2_5_mixed_family_campaign,
    compile_capability_receipt, project_matter_issue_workspace, project_matter_issue_workbench,
    AustralianCalibrationKind, MatterEntityKind, MatterEntityProjection, MatterEventProjection,
    MatterWorkbenchSeed, MixedFamilyReplayReceipt,
};

fn runtime_error(label: &str, error: impl std::fmt::Debug) -> std::io::Error {
    std::io::Error::other(format!("{label}: {error:?}"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("artifacts/legal-runtime-capstone"));

    fs::create_dir_all(&output)?;

    let mixed = build_m2_5_mixed_family_campaign()
        .map_err(|error| runtime_error("M2.5 campaign failed", error))?;
    let mixed_payload = mixed.encode();
    fs::write(output.join("m2_5_mixed_family_replay.tsv"), &mixed_payload)?;
    let mixed_reloaded = MixedFamilyReplayReceipt::decode(
        &fs::read_to_string(output.join("m2_5_mixed_family_replay.tsv"))?,
    )
    .map_err(|error| runtime_error("M2.5 reload failed", error))?;
    mixed
        .validate_exact_replay(&mixed_reloaded)
        .map_err(|error| runtime_error("M2.5 replay identity failed", error))?;

    let mut report = Vec::new();
    for kind in [
        AustralianCalibrationKind::Mabo,
        AustralianCalibrationKind::Pabai,
        AustralianCalibrationKind::CullenNswCla,
        AustralianCalibrationKind::Glj,
    ] {
        let capstone = build_australian_calibration_capstone(kind)
            .map_err(|error| runtime_error(&format!("{kind:?} capstone failed"), error))?;
        capstone
            .campaign
            .validate_restart_replay()
            .map_err(|error| runtime_error(&format!("{kind:?} replay failed"), error))?;

        let campaign_payload = capstone.campaign.encode();
        let campaign_path =
            output.join(format!("{kind:?}.legal-campaign.tsv").to_lowercase());
        fs::write(&campaign_path, campaign_payload)?;
        let reloaded_campaign = fs::read_to_string(&campaign_path)?;
        capstone
            .campaign
            .validate_persisted_payload(&reloaded_campaign)
            .map_err(|error| runtime_error(&format!("{kind:?} disk replay failed"), error))?;

        let last = capstone
            .campaign
            .hops
            .last()
            .ok_or_else(|| std::io::Error::other("capstone has no campaign hop"))?;
        let workspace =
            project_matter_issue_workspace(format!("matter:{kind:?}"), &capstone.issue, last);

        let first_evidence = capstone
            .issue
            .elements
            .iter()
            .flat_map(|element| element.evidence.iter())
            .next()
            .ok_or_else(|| std::io::Error::other("capstone has no source-addressable evidence"))?;
        let workbench = project_matter_issue_workbench(
            format!("matter:{kind:?}"),
            &capstone.issue,
            last,
            MatterWorkbenchSeed {
                entities: vec![MatterEntityProjection {
                    entity_ref: format!("entity:{kind:?}:party"),
                    label: format!("{kind:?} calibration party"),
                    kind: MatterEntityKind::Person,
                    source_revision_refs: vec![first_evidence.source_revision_ref.clone()],
                    candidate_only: true,
                }],
                events: vec![MatterEventProjection {
                    event_ref: format!("event:{kind:?}:reviewed"),
                    label: format!("{kind:?} reviewed event"),
                    time_ref: "2026-09-20T00:00:00+10:00".into(),
                    observation_refs: vec![first_evidence.observation_ref.clone()],
                    entity_refs: vec![format!("entity:{kind:?}:party")],
                    candidate_only: true,
                }],
                observation_time_refs: std::collections::BTreeMap::from([(
                    first_evidence.observation_ref.clone(),
                    "2026-09-20T00:00:00+10:00".into(),
                )]),
            },
        )
        .map_err(|error| std::io::Error::other(format!("M4.A workbench failed: {error}")))?;
        fs::write(
            output.join(format!("{kind:?}.m4a-workbench.txt").to_lowercase()),
            format!("{workbench:#?}"),
        )?;

        report.push(format!(
            "{kind:?}\tapplicability={:?}\tviolation={:?}\tliability={:?}\tremedy={:?}\tresiduals={}\tnodes={}\tentities={}\tobservations={}\tevents={}\tdocuments={}\ttimeline={}\treceipt_head={}",
            workspace.applicability,
            workspace.violation,
            workspace.liability,
            workspace.remedy,
            last.residuals.len(),
            workspace.nodes.len(),
            workbench.entities.len(),
            workbench.observations.len(),
            workbench.events.len(),
            workbench.documents.len(),
            workbench.timeline.len(),
            capstone.campaign.receipt_head,
        ));
    }

    let receipt = compile_capability_receipt()
        .map_err(|error| runtime_error("capability receipt failed", error))?;
    report.push(format!(
        "capability\tm2_5={}\tm3_a={}\tm3_b={}\tm3_c={}\tm3_c_replay={}\tm4_a={}\tcandidate_only={}\tsemantic_authority={}\tdigest={}",
        receipt.m2_5_mixed_family_replay,
        receipt.m3_a_reviewed_world_to_wrong_type,
        receipt.m3_b_source_realised_evaluator,
        receipt.m3_c_all_calibrations_one_runner,
        receipt.m3_c_restart_replay,
        receipt.m4_a_matter_issue_projection,
        receipt.candidate_only,
        receipt.creates_semantic_authority,
        receipt.receipt_digest,
    ));

    fs::write(output.join("capability-report.tsv"), report.join("\n"))?;
    println!("{}", report.join("\n"));
    Ok(())
}
