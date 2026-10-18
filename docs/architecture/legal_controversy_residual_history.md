# Legal controversy residual history and current projection

Date: 2026-10-17

A stable REAL-MATTER controversy may accumulate more than one immutable residual row when a reviewed dispute narrative is refined. Historical residuals are evidence about prior represented state; they are never overwritten merely because a later narrative exists.

The operator/reverse-search projection therefore obeys:

```text
all immutable residual rows
+ explicit reviewed supersession receipts
-> unique unsuperseded residual head per typed lane
-> current Matter/reverse-search projection
```

The historical `load_legal_controversy_matter` loader remains unchanged and exposes all residual/obligation rows for audit and reconstruction. `load_current_legal_controversy_matter` is the production/operator projection and filters historical residual-owned obligations after deriving current heads.

## No implicit recency

Current meaning is never selected from insertion order, lexical hash order, timestamps, or a caller's latest string. Multiple unsuperseded residuals in one lane fail closed with `AmbiguousResidualHead`.

A residual lane is identified by:

```text
controversy
+ disagreement kind
+ applicant fibre
+ respondent fibre
```

Narrative wording, discriminator and evidence query are intentionally excluded: changing those coordinates is precisely the refinement being reviewed.

## Reviewed supersession

`ResidualSupersessionReceipt` is immutable and must bind:

- one existing controversy;
- one existing successor residual;
- one or more existing predecessor residuals in the same typed lane;
- an existing S29 `semantic.review_receipt` whose `semantic_ref` is the successor residual and whose action explicitly accepts, qualifies or supersedes that reviewed stage.

It creates no semantic/legal authority, applicability or claim truth.

A single reviewed successor may reconcile multiple historical unsuperseded heads. This is useful for legacy data that accumulated more than one residual before the current-head invariant existed; all predecessors remain historically reopenable.

## Replay invariant

The intended commuting property is:

```text
persisted historical children + reviewed supersession
        |                         |
        v                         v
 derive current projection -> strict REAL-MATTER replay
```

Strict replay and reverse proof search consume the current projection. Historical reconstruction consumes the audit loader. A residual refinement therefore changes current operator meaning only through an explicit reviewed receipt, never through destructive update or silent cache replacement.
