use sensiblaw_legal_runtime::{
    run_pabai_comparative_regression, workbench_overlay_from_explanation,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let receipt = run_pabai_comparative_regression()?;
    let d = workbench_overlay_from_explanation(&receipt.w0_to_w1_explanation)?;
    let c = workbench_overlay_from_explanation(&receipt.w1_to_w2_explanation)?;

    println!("--- w0-to-w1 ---");
    println!("{}", serde_json::to_string_pretty(&d)?);
    println!("--- w1-to-w2 ---");
    println!("{}", serde_json::to_string_pretty(&c)?);
    Ok(())
}
