use std::{env, process};

use sensiblaw_legal_runtime::{
    run_pabai_comparative_regression, workbench_overlay_from_explanation,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let which = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: m11_pabai_workbench_overlays <D|C>");
        process::exit(2);
    });

    let receipt = run_pabai_comparative_regression()?;
    let overlay = match which.as_str() {
        "D" | "d" => workbench_overlay_from_explanation(&receipt.w0_to_w1_explanation)?,
        "C" | "c" => workbench_overlay_from_explanation(&receipt.w1_to_w2_explanation)?,
        _ => {
            eprintln!("usage: m11_pabai_workbench_overlays <D|C>");
            process::exit(2);
        }
    };

    println!("{}", serde_json::to_string_pretty(&overlay)?);
    Ok(())
}
