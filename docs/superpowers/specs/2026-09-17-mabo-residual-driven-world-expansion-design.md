# Mabo Residual-Driven World Expansion Design

## Goal

Starting from the Mabo legal/identity seed surface, admit 100 novel, reviewed knowledge objects (QIDs, articles, primary/legal source manifestations, or other typed world objects) by repeatedly routing current PNF/world residuals to the producer most likely to contract those residuals.

`100` is an output cardinality target, not traversal depth.

## Existing canonical substrate

The implementation MUST reuse rather than replace:

- `sl-consumer-residual`: typed PNF/evidence requirements, gaps, obligations, and payments.
- `sl-proof-search-loop::frontier`: open `ProofResidual`s, whole-frontier reduction, shared-dependency gain, Pareto selection.
- `sl-proof-search-scheduler`: cost/value vectors and candidate-only move receipts.
- `sl-legal-follow-plan`: legal-source demands and OALC/provider-facing acquisition planning.
- `sl-governed-legal-provider`: governed legal acquisition.
- `sl-wikimedia-candidate-provider` + route selection/execution: identity/article candidate production.
- `sl-proof-search-loop::world`: append-only source revisions and learned world vocabulary/authority neighbourhood.
- `sl-pg-source-store`: reviewed context persistence and latent-world materialisation.

No new crawler, ontology, truth rank, or semantic-authority mechanism is introduced.

## Control loop

```text
PNF / consumer requirements
        -> current world
        -> open residual frontier
        -> quotient against already-known/admitted objects
        -> residual routing context
        -> producer candidate moves
        -> residual-sensitive Pareto selection
        -> governed acquisition
        -> parse / PNF
        -> world / same-object disambiguation
        -> review
        -> persist admitted object + lineage
        -> recompute residual frontier
        -> repeat until 100 novel admitted objects
```

The question/gap is the unit of recursion. Hyperlinks, Wikidata adjacency, citation links, and first-link edges are candidate producers/navigation priors only.

## Residual-sensitive producer routing

Producer preference is not a global ordering.

For each residual `r` in world `w`, selection prioritises expected residual contraction first, with residual-domain fit as a typed tie-break / Pareto coordinate.

Canonical lane priors:

- legal/doctrinal/authority/source-text residual -> governed legal lane first-refusal (`sl-legal-follow-plan`, OALC/official legal provider);
- identity/entity/same-object residual -> Wikidata identity lane;
- explanatory/context/article residual -> Wikipedia/article-semantic lane;
- provenance/source-history residual -> source-specific provenance lane;
- otherwise -> existing frontier/Pareto machinery without a hard-coded producer winner.

Legal-first therefore means legal-residual-first. A legal acquisition is followed recursively only while it contracts an open legal residual or exposes a higher-value legal residual after quotienting against the current world.

## World-disambiguation gate

Acquisition does not imply admission. Each candidate must record a disambiguation outcome before persistence:

- `SameObject`
- `NewRelatedObject`
- `NewSourceManifestation`
- `NewConceptualParent`
- `NewEvidentiarySource`
- `Ambiguous`
- `WrongType`
- `Duplicate`
- `IrrelevantToResidual`

Only the first five are admission-capable, and review remains explicit. QID/article/source identity never manufactures authority, applicability, proof payment, or claim truth.

## Novel-object budget

The default target is exactly 100 novel admitted objects.

Count separately:

- candidates seen;
- candidates rejected;
- duplicates seen;
- ambiguous identities;
- reviewed/admitted objects;
- new QIDs;
- new articles;
- new primary/legal source manifestations;
- other new world objects.

An object counts once by canonical admitted object identity. Multiple source revisions or edges may support one object without incrementing object cardinality unless the source manifestation itself is explicitly admitted as a distinct typed knowledge object.

Success is `total_new_world_objects >= target_novel_objects`; depth is only an observed topology metric.

## Discovery lineage receipt

Every admitted object retains:

- object identity and kind;
- discovery parent;
- triggering residual;
- PNF/evidence obligation class supplied by the caller;
- producer lane and source revision/manifestation when applicable;
- selected move / routing reason;
- disambiguation outcome;
- residual-contraction estimate used at selection time;
- post-admission world delta / residuals contracted when observed;
- admission/rejection status.

The receipt is explanatory/provenance metadata, not truth or authority.

## Mabo acceptance target

Seed surface:

```text
mabo:proposition:radical-title-native-title
Q1501525
canonical [1992] HCA 23 source identity when resolved
```

Target receipt:

```text
target_novel_objects = 100
total_new_world_objects >= 100
```

with provenance and discovery lineage for every admitted object, no duplicate counting, explicit unresolved ambiguity, and non-promotion invariants retained.

The output may be broad and shallow; 100 useful objects at depth 3 is preferable to an arbitrary chain of depth 100.

## Formal boundaries

```text
100 objects != 100 hops
100 objects != 100 paid propositions
100 objects != 100 authorities
producer preference != source authority
source authority != claim truth
candidate reachability != admission
disambiguation != proof payment
QID/article identity != semantic authority
first-link/adjacency != semantic controller
```

## Agda parity

Add a thin theorem-bearing owner over existing PNF/Ibrahim/Wikimedia review machinery. It should formalise:

- target cardinality vs traversal depth are independent coordinates;
- producer priority is residual-indexed rather than globally ordered;
- legal-first does not imply legal-only;
- admission requires disambiguation/review and cannot factor through reachability;
- QID/article/source identity cannot manufacture proof, applicability, authority, or truth;
- Ibrahim acts as coverage/frontier policy over residuals, not as a first-link traversal mechanism.
