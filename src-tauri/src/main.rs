#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::select_file,
            commands::detect_source_type,
            commands::run_mask,
            commands::export_masked_csv,
            commands::load_preview,
            commands::apply_rules,
            commands::export_selected_csv,
            commands::apply_rules_cols,
            commands::run_validate,
            commands::export_records_csv,
            commands::export_records_xlsx,
            commands::save_ruleset,
            commands::read_ruleset,
            commands::preview_mask_rule,
            commands::preview_validate_rule,
            commands::preview_mask_rule_value,
            commands::preview_validate_rule_value,
            commands::list_mask_op_types,
            commands::list_validate_op_types,
            commands::scan_log_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running RuT0DataKit GUI");
}
