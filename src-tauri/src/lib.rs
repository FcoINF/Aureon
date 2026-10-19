//! Aureon · Aplicación de finanzas personales.
//!
//! La interfaz (React) nunca habla con la base de datos directamente: todos
//! los accesos pasan por comandos que validan las reglas financieras en Rust.

pub mod commands;
pub mod core;
pub mod state;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            std::fs::create_dir_all(&data_dir).ok();

            let state = AppState::new(data_dir);
            // Si la base de datos existe, se abre y se migra. Si no existe,
            // la aplicación se puede usar igual y el archivo se creará
            // cuando el usuario guarde su primera operación.
            if let Err(error) = state.init() {
                eprintln!("[aureon] no se pudo abrir la base de datos: {error}");
            }
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::accounts::list_currencies,
            commands::accounts::list_accounts,
            commands::accounts::create_account,
            commands::accounts::update_account,
            commands::accounts::set_account_archived,
            commands::accounts::delete_account,
            commands::accounts::list_categories,
            commands::accounts::create_category,
            commands::accounts::update_category,
            commands::accounts::set_category_archived,
            commands::accounts::list_contacts,
            commands::accounts::create_contact,
            commands::accounts::update_contact,
            commands::accounts::delete_contact,
            commands::transactions::list_transactions,
            commands::transactions::count_transactions,
            commands::transactions::create_transaction,
            commands::transactions::update_transaction,
            commands::transactions::void_transaction,
            commands::transactions::unvoid_transaction,
            commands::transactions::delete_transaction,
            commands::purchases::list_purchases,
            commands::purchases::get_purchase,
            commands::purchases::list_installments,
            commands::purchases::create_purchase,
            commands::purchases::pay_installment,
            commands::purchases::update_installment,
            commands::purchases::cancel_purchase,
            commands::purchases::refund_purchase,
            commands::purchases::list_budgets,
            commands::purchases::save_budget,
            commands::purchases::delete_budget,
            commands::subscriptions::list_subscriptions,
            commands::subscriptions::get_subscription,
            commands::subscriptions::list_subscription_payments,
            commands::subscriptions::create_subscription,
            commands::subscriptions::update_subscription,
            commands::subscriptions::charge_subscription,
            commands::subscriptions::set_subscription_status,
            commands::subscriptions::delete_subscription,
            commands::subscriptions::list_subscription_shares,
            commands::subscriptions::add_subscription_share,
            commands::subscriptions::remove_subscription_share,
            commands::obligations::list_receivables,
            commands::obligations::get_receivable,
            commands::obligations::list_receivable_payments,
            commands::obligations::create_receivable,
            commands::obligations::update_receivable,
            commands::obligations::receive_money,
            commands::obligations::set_receivable_status,
            commands::obligations::delete_receivable,
            commands::obligations::list_payables,
            commands::obligations::get_payable,
            commands::obligations::list_payable_payments,
            commands::obligations::create_payable,
            commands::obligations::update_payable,
            commands::obligations::pay_obligation,
            commands::obligations::set_payable_status,
            commands::obligations::delete_payable,
            commands::obligations::list_deposits,
            commands::obligations::get_deposit,
            commands::obligations::create_deposit,
            commands::obligations::update_deposit,
            commands::obligations::complete_deposit,
            commands::obligations::set_deposit_status,
            commands::obligations::delete_deposit,
            commands::wishlist::list_wishlist,
            commands::wishlist::create_wishlist_item,
            commands::wishlist::update_wishlist_item,
            commands::wishlist::set_wishlist_status,
            commands::wishlist::convert_wishlist_item,
            commands::wishlist::delete_wishlist_item,
            commands::reports::dashboard_summary,
            commands::reports::summary_by_currency,
            commands::reports::income_expense_series,
            commands::reports::expenses_by_category,
            commands::reports::expenses_by_contact,
            commands::reports::balance_evolution,
            commands::reports::history_summary,
            commands::reports::recent_transactions,
            commands::reports::upcoming_commitments,
            commands::reports::calendar_events,
            commands::reports::list_calendar_events,
            commands::reports::search_transactions,
            commands::system::get_settings,
            commands::system::save_settings,
            commands::system::app_info,
            commands::system::refresh_overdue_status,
            commands::system::export_dataset,
            commands::system::write_text_file,
            commands::system::write_base64_file,
            commands::system::create_backup,
            commands::system::validate_backup_file,
            commands::system::restore_backup,
            commands::system::import_attachment,
            commands::system::open_attachment,
            commands::system::open_external_url,
            commands::system::default_backup_path,
            commands::system::app_data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar Aureon");
}
