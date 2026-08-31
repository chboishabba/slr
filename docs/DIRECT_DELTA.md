# Direct delta compiler tranche

This tranche implements the runtime shape constrained by the DASHI direct-delta constitution.

## Sentence-local carrier

spaCy observations are packed into columnar `PackedSentence` storage. `FibreAddress` is the canonical local execution coordinate `(sentence_id, local_ordinal)`. Head projection and role extraction occur entirely in memory.

The mandatory sentence path exposes no database API. Its physical counters therefore remain:

- sentence-local DB crossings: `0`
- production parser-token writes: `0`
- unchanged relation writes: `0`
- parent reads of closed-child interiors: `0`

## Delta and residual split

A supported dependency produces a `NormativeDelta` attached to `StableSourceEvidence` (revision-scoped text span + fibre address). Unsupported parser relations or projection failures remain `SemanticResidual`s. A residual is not silently promoted into semantic truth.

## Natural transport

The concrete restriction map projects a sentence state to `RoleCounts`. A child `NormativeDelta` transports to a parent role-count delta, and `transport_commutes` checks the commuting square:

`restrict(apply_child(state, delta)) == apply_parent(restrict(state), transport(delta))`

Paragraph fusion accepts only `SentenceOutwardDelta`; sentence interior is absent from the API. This makes ordinary closed-child rescanning structurally unavailable.

## Generation publication

`GenerationPublisher` is append-only. Staging records candidate counts but does not alter the current consumer-visible generation. `publish` fails closed unless the generation has an explicit certification receipt first. The parser stream therefore stages a generation and deliberately leaves `published=0`.

This preserves the authority boundary:

`parser observation -> candidate generation != published semantic authority`.
