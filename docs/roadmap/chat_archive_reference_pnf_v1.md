# Canonical chat archive → SLR reference materialisation v1

## Contract and TODO (before implementation)

The canonical re-gpt SQLite archive alone owns raw ChatGPT message bodies. PostgreSQL owns only source identity, canonical archive locator, SHA-256 digest, provenance coordinates and candidate-only selected-span/PNF products. A reference-only chat source row has no `corpus.canonical_content.payload`, `corpus.document` or `corpus.span` materialisation. Never route reference-only rows through the legacy `persist_chat_archive_message` path, which stores raw bodies.

Stable identity is `message_ref` scoped to the canonical `conversation_ref`, `node_ref` and immutable source thread identity. Equal identity+digest is a no-op. Changed digest is a recorded conflict requiring explicit supersession/review, not overwrite. Parent/node, role, branch, timestamp, attachment and provenance remain coordinates, not truth. The consumer must reopen the explicit canonical archive locator and verify SHA-256 before bounded selected-span parsing. The existing parser and candidate-PNF producers own compilation; no implicit full-thread parsing, semantic admission or promotion.

TODO: [ ] implement producer-owned bounded canonical archive reopen with exact digest check and selected-span parser handoff without PG body persistence; [ ] enforce read-only world-packet projection and receipt joins; [ ] independently run PG-backed integration tests. This cut must not imply a verified PNF reduction merely from reference registration.
