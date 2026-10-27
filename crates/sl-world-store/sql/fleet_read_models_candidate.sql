-- Candidate-only fleet read models over native SLR tables.
-- IMPORTANT: These views do not promote semantic or proof authority.
-- Exact source joins require a verified producer-owned source identity.
-- PostgreSQL cannot decode PNF1/OBL1 binary payloads by itself.
CREATE OR REPLACE VIEW public.v_fleet_core_knowledge_spine AS
SELECT
  p.claim_candidate_id AS candidate_ref,
  p.source_manifestation_id AS source_manifestation_ref,
  p.created_at AS persisted_at,
  p.payload AS native_pnf1_payload,
  'candidate_only'::text AS knowledge_status,
  COALESCE(src.source_coordinates, '[]'::jsonb) AS source_coordinates
FROM public.slr_world_v2_pnf_candidate AS p
LEFT JOIN LATERAL (
  SELECT jsonb_agg(
    jsonb_build_object(
      'message_ref', c.message_ref,
      'conversation_ref', c.conversation_ref,
      'node_ref', c.node_ref,
      'body_storage_ref', c.body_storage_ref,
      'source_revision_ref', c.source_revision_ref,
      'identity_verified', c.identity_verified
    ) ORDER BY c.message_ref
  ) AS source_coordinates
  FROM corpus.chat_archive_message AS c
  WHERE c.document_ref = p.source_manifestation_id
    AND c.identity_verified IS TRUE
) AS src ON TRUE;

-- Current native obligation observations; PAY1/PAY2 eligibility must be
-- validated by the native evidence-payment kernel before treating any payment
-- as formal discharge. The existing native frontier SQL suppresses by exact
-- target and iteration, but does not itself certify proof authority.
CREATE OR REPLACE VIEW public.v_fleet_actionable_gap_frontier AS
WITH latest AS (
  SELECT o.obligation_id, max(o.iteration_index) AS iteration_index
  FROM public.slr_world_v2_obligation o
  GROUP BY o.obligation_id
)
SELECT
  o.obligation_id,
  o.iteration_index,
  o.obligation_kind,
  o.payload AS native_obligation_payload,
  COALESCE(p.payment_receipts, '[]'::jsonb) AS payment_receipts,
  COALESCE(f.formal_demands, '[]'::jsonb) AS formal_demands,
  (p.payment_receipts IS NULL) AS no_recorded_payment_at_or_after
FROM latest l
JOIN public.slr_world_v2_obligation o
  ON o.obligation_id = l.obligation_id
 AND o.iteration_index = l.iteration_index
LEFT JOIN LATERAL (
  SELECT jsonb_agg(jsonb_build_object(
      'payment_id', p.payment_id,
      'iteration_index', p.iteration_index
    ) ORDER BY p.iteration_index, p.payment_id) AS payment_receipts
  FROM public.slr_world_v2_payment p
  WHERE p.target_residual_id = o.obligation_id
    AND p.iteration_index >= o.iteration_index
) p ON TRUE
LEFT JOIN LATERAL (
  SELECT jsonb_agg(jsonb_build_object(
      'demand_ref', d.demand_ref,
      'statement_digest', d.statement_digest,
      'target_prover', d.target_prover
    ) ORDER BY d.demand_ref) AS formal_demands
  FROM formal_bridge.demand d
  WHERE d.obligation_ref = o.obligation_id
) f ON TRUE;
