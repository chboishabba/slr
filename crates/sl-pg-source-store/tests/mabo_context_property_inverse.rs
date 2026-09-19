use sensiblaw_pg_source_store::mabo_wikidata_property_ref;

#[test]
fn reviewed_mabo_context_relation_types_invert_to_exact_property_ids() {
    assert_eq!(mabo_wikidata_property_ref("context:wikidata:jurisdiction"), Some("P1001"));
    assert_eq!(mabo_wikidata_property_ref("context:wikidata:participant"), Some("P710"));
    assert_eq!(mabo_wikidata_property_ref("context:wikidata:court"), Some("P4884"));
    assert_eq!(mabo_wikidata_property_ref("context:wikidata:judge"), Some("P1594"));
    assert_eq!(mabo_wikidata_property_ref("context:wikidata:overrules"), Some("P4006"));
}

#[test]
fn arbitrary_context_label_has_no_provider_property_inverse() {
    assert_eq!(mabo_wikidata_property_ref("context:wikidata:made-up"), None);
    assert_eq!(mabo_wikidata_property_ref("legal_ir:pnf_factor"), None);
}
