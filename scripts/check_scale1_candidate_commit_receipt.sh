#!/usr/bin/env bash
set -euo pipefail

receipt="${1:?usage: check_scale1_candidate_commit_receipt.sh <compiler-receipt.json>}"

jq -e '
  (.compiler_receipt // .) as $r
  | ($r.integrity // $r) as $i
  | ($r.performance // $r) as $p
  | if ($i.candidate_persistence_reused // false) then
      ($i.candidate_commit_count == 0)
      and (($p.candidate_precommit_write_ns // 0) == 0)
      and (($p.candidate_commit_wait_ns // 0) == 0)
      and (($p.candidate_postcommit_reopen_ns // 0) == 0)
    else
      ($i.candidate_commit_batch_size > 0)
      and ($i.candidate_commit_count > 0)
      and ($i.candidate_commit_count
           == (((($i.persisted_candidate_batch_count - 1)
                 / $i.candidate_commit_batch_size) | floor) + 1))
      and ($i.candidate_commit_count < $i.persisted_candidate_batch_count)
      and (($p.candidate_precommit_write_ns // -1) >= 0)
      and (($p.candidate_commit_wait_ns // -1) >= 0)
      and (($p.candidate_postcommit_reopen_ns // -1) >= 0)
      and (
        (($p.candidate_precommit_write_ns // 0)
         + ($p.candidate_commit_wait_ns // 0)
         + ($p.candidate_postcommit_reopen_ns // 0))
        <= $p.candidate_persist_ns
      )
    end
  and ($i.candidate_pnf_reopen_complete == true)
  and ($i.creates_semantic_authority == false)
  and ($i.applicability_promoted == false)
  and ($i.claim_truth_promoted == false)
  and (($p.candidate_commit_batch_size // $i.candidate_commit_batch_size)
       == $i.candidate_commit_batch_size)
  and (($p.candidate_commit_count // $i.candidate_commit_count)
       == $i.candidate_commit_count)
' "$receipt"

echo "SCALE1_CANDIDATE_COMMIT_ECONOMY_GREEN"
jq '
  (.compiler_receipt // .) as $r
  | ($r.integrity // $r) as $i
  | ($r.performance // $r) as $p
  | {
      candidate_persistence_reused: $i.candidate_persistence_reused,
      candidate_batches: $i.persisted_candidate_batch_count,
      commit_batch_size: $i.candidate_commit_batch_size,
      commit_count: $i.candidate_commit_count,
      candidate_persist_ns: $p.candidate_persist_ns,
      candidate_precommit_write_ns: $p.candidate_precommit_write_ns,
      candidate_commit_wait_ns: $p.candidate_commit_wait_ns,
      candidate_postcommit_reopen_ns: $p.candidate_postcommit_reopen_ns,
      candidate_mean_commit_wait_ns: $p.candidate_mean_commit_wait_ns,
      exact_reopen: $i.candidate_pnf_reopen_complete,
      creates_semantic_authority: $i.creates_semantic_authority,
      applicability_promoted: $i.applicability_promoted,
      claim_truth_promoted: $i.claim_truth_promoted
    }
' "$receipt"
