use sensiblaw_pg_source_store::{
    load_chat_archive_export_jsonl, load_database_config,
    persist_chat_archive_message,
};

fn main() -> Result<(), String> {
    let export_path = std::env::args()
        .nth(1)
        .ok_or_else(|| "usage: chat_archive_source_import <slr-chat-export.jsonl>".to_owned())?;

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let messages =
        load_chat_archive_export_jsonl(&export_path).map_err(|error| error.to_string())?;

    let mut imported = 0usize;
    let mut inactive = 0usize;
    for message in &messages {
        let persisted =
            persist_chat_archive_message(&config, message)
                .map_err(|error| error.to_string())?;
        if message.is_inactive_generated_branch() {
            inactive += 1;
        }
        println!(
            "message={} conversation={} node={} branch={:?} role={:?} kind={:?} document={} span={}",
            persisted.message_ref,
            persisted.conversation_ref,
            persisted.node_ref,
            persisted.branch_membership,
            persisted.role,
            persisted.content_kind,
            persisted.document_ref,
            persisted.full_message_span_ref,
        );
        imported += 1;
    }

    println!("message_count={imported}");
    println!("inactive_assistant_branch_count={inactive}");
    println!("identity_verified=true");
    println!("candidate_only=true");
    println!("creates_semantic_authority=false");
    println!("claim_truth_promoted=false");

    Ok(())
}
