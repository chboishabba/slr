# Atomic attribution parity

This tranche is the SLR runtime projection of the SensibLaw Agda atomic/source-conditioned attribution spine.

It is intentionally not a second legal theorem engine. Agda remains the proof/specification owner for source-conditioned rule application, WrongType/element derivation, applicability, violation and typed liability. SLR owns finite execution/persistence/admission carriers.

## Runtime boundary

The runtime carrier preserves, for one exact case/revision atom:

```text
source artifact/span
  -> admitted semantic candidate
  -> legal atom definition lineage
  -> case-outcome evidence lineage
  -> repository evaluation lineage
  -> atomic gate (-1 / 0 / +1)
  -> finite same-case registry
  -> consumer-relative acquisition admission
  -> separately governed promotion
```

The three attribution fibres are separate:

1. **definition lineage** — provenance of the repository proposition that defines the legal atom;
2. **outcome lineage** — provenance of the case-specific evidence proposition used to evaluate that atom;
3. **evaluation lineage** — provenance of the repository operation that assigns the atomic fit state.

A primary statute or judgment may support a repository reconstruction. Primary-source support does not make the Rust/Agda proposition object an externally authored formal proposition.

## Atomic semantics

The runtime enum is intentionally proposition-relative:

```text
FailsThisAtom       = -1
UnresolvedThisAtom  =  0
FitsThisAtom        = +1
```

`FailsThisAtom(A)` is a positive recorded failure of the exact test `A`. It is not a proof of an antonym/opposite proposition and it does not aggregate with another negative atom into a generic negative legal truth.

A resolved +/- atom requires outcome-evidence lineage. An unresolved atom must not invent outcome evidence.

## Same-case coherence

`AtomicCaseRegistry` has one canonical entry per:

```text
(case_context, atom_id)
```

Reinserting the identical entry is idempotent. A different gate or provenance payload for the same key is a conflict. A changed evidential state therefore needs a new revision/case-context lineage rather than silent overwrite.

## Candidate/source weld

`admission::SourceSpanAnchor` carries:

- source id;
- source revision;
- exact locator;
- stable candidate `FibreAddress`;
- exact revision-scoped `TextSpan`.

`weld_admitted_candidate_to_source` succeeds only when the already-admitted semantic delta has the same stable address and source span. Matching locator text alone is insufficient.

The weld creates no legal atom gate, truth, authority, applicability or promotion.

## Semantic status remains orthogonal

The atomic fixture reuses `sensiblaw-semantic-status`. Primary-source support does not silently change:

- proposition status;
- truth status;
- legal authority;
- applicability;
- violation;
- liability.

Those remain independently governed runtime coordinates.

## Cullen bounded gold regression

The current bounded fixture is:

```text
Cullen v New South Wales [2026] HCA 19
case context: case:Cullen:[2026]HCA19:retained-fibre:r1

s 5B(1)(a) foreseeable risk                              +1
s 5B(1)(b) risk not insignificant                       +1
s 5B(1)(c) reasonable person would take proposed steps  -1
s 43A special-statutory-power engagement                 -1
vicarious-liability family recognised                    +1
```

The reviewable fixture is `fixtures/cullen_atomic_attribution_v0_1.tsv`. It pins the definition source, exact locator/role, outcome-evidence source, and provenance stages for all five atoms.

This is a parity fixture for already reviewed source/Agda conclusions. It is not a claim that the current parser independently discovers those five legal atoms from raw judgment text.

The two `-1` coordinates retain different proposition identities and different legal meanings. The `+1` vicarious-family coordinate does not repair the failed breach atom.

## Consumer-relative acquisition

`priority` reuses the existing `sensiblaw-proof-search-scheduler`; it does not create another planner.

An unresolved source coordinate is first classified as:

```text
AcquireForLiveConsumer
DeferForCounterfactualConsumer
BlockedDownstream
OptionalEnrichment
```

A `BlockedDownstream` demand must name a registered `FailsThisAtom` blocker. Only `AcquireForLiveConsumer` becomes a scheduler `ProofGap`.

The Cullen regression uses the same unresolved s 8(a)/(b) vicarious-liability subroute twice:

- actual no-liability disposition: blocked downstream by registered s 5B(1)(c) failure;
- explicit counterfactual "if the police tort existed, which s 8 route applies?": live source-acquisition gap.

Thus `unresolved != acquire now`, and a coordinate can be deferred for one consumer while live for another.

## Promotion

Atomic attribution reuses the canonical `sensiblaw_core::PromotionReceipt`; there is no second promotion authority type.

`AtomicPromotionWeld` adds only the missing atom/context/provenance identity and exact admitted-source weld. Promotion succeeds only when:

- the target atom exists in the exact case registry;
- the stated prior provenance stage matches the registered evaluation stage;
- the core receipt has `PromotionStatus::Promoted`;
- the core receipt source span equals the admitted/source anchor span;
- policy and reviewer/resolver references are present.

The promotion receipt does not change the atomic gate and does not manufacture legal authority, applicability, violation or liability.

## Validation target

The focused runtime target is:

```sh
cargo test -p sensiblaw-atomic-attribution
cargo run -p sensiblaw-atomic-attribution --example cullen_atomic
cargo clippy -p sensiblaw-atomic-attribution --all-targets -- -D warnings
```

Workspace CI continues to be the broader integration gate.
