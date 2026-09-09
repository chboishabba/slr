use sensiblaw_atomic_attribution::{AtomicGate, CULLEN_CONTEXT, cullen_gold_registry};

fn gate_symbol(gate: AtomicGate) -> &'static str {
    match gate {
        AtomicGate::FailsThisAtom => "-1",
        AtomicGate::UnresolvedThisAtom => "0",
        AtomicGate::FitsThisAtom => "+1",
    }
}

fn main() {
    let registry = cullen_gold_registry();
    let atoms = [
        "atom:NSW:CLA:s5B1a:risk-foreseeable",
        "atom:NSW:CLA:s5B1b:risk-not-insignificant",
        "atom:NSW:CLA:s5B1c:reasonable-person-would-take-proposed-precautions",
        "atom:NSW:CLA:s43A:liability-based-on-special-statutory-power",
        "atom:NSW:vicarious-liability:family-recognised",
    ];

    println!("CULLEN_ATOMIC context={CULLEN_CONTEXT} entries={}", registry.len());
    for atom in atoms {
        let entry = registry
            .get(CULLEN_CONTEXT, atom)
            .expect("bounded Cullen atom must be registered");
        println!(
            "CULLEN_ATOMIC atom={} gate={} definition_stage={:?} evaluation_stage={:?}",
            atom,
            gate_symbol(entry.gate),
            entry.definition_lineage.stage,
            entry.evaluation_lineage.stage,
        );
    }
}
