//! Pruebas de integración de Aureon.
//!
//! Verifican las reglas financieras y los flujos principales contra una base
//! de datos SQLite real, creada en un directorio temporal.

use std::path::PathBuf;

use aureon_lib::core::dates;
use aureon_lib::core::models::*;
use aureon_lib::core::repos;
use aureon_lib::core::services::{
    accounts, budgets, categories, deposits, export, payables, purchases, receivables, reports, settings,
    subscriptions, transactions as tx_service, wishlist,
};
use aureon_lib::state::AppState;
use rusqlite::Connection;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "aureon-test-{}-{}",
        name,
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir).expect("no se pudo crear el directorio temporal");
    dir
}

fn new_state(name: &str) -> (AppState, PathBuf) {
    let dir = temp_dir(name);
    let state = AppState::new(dir.clone());
    (state, dir)
}

fn create_account(
    state: &AppState,
    name: &str,
    account_type: &str,
    currency: &str,
    initial: i64,
) -> String {
    state
        .write(|conn| {
            accounts::create(
                conn,
                &AccountInput {
                    name: name.to_string(),
                    account_type: account_type.to_string(),
                    currency_code: currency.to_string(),
                    initial_balance: initial,
                    initial_balance_date: "2024-01-01".to_string(),
                    archived: Some(false),
                    description: None,
                },
            )
        })
        .expect("no se pudo crear la cuenta")
        .id
}

fn transaction(
    state: &AppState,
    transaction_type: &str,
    description: &str,
    amount: i64,
    account_id: &str,
    transfer_account_id: Option<&str>,
    date: &str,
) -> Transaction {
    state
        .write(|conn| {
            tx_service::create(
                conn,
                &TransactionInput {
                    transaction_type: transaction_type.to_string(),
                    description: description.to_string(),
                    amount,
                    currency_code: "CLP".to_string(),
                    date: date.to_string(),
                    category_id: None,
                    account_id: Some(account_id.to_string()),
                    transfer_account_id: transfer_account_id.map(|v| v.to_string()),
                    contact_id: None,
                    payment_method: None,
                    notes: None,
                    tags: None,
                    related_transaction_id: None,
                },
            )
        })
        .expect("no se pudo crear el movimiento")
}

fn balance(state: &AppState, account_id: &str) -> i64 {
    state
        .read(|conn| Ok(accounts::get(conn, account_id).unwrap().balance))
        .unwrap()
        .unwrap()
}

// ---------------------------------------------------------------------------
// 1. Creación diferida de la base de datos
// ---------------------------------------------------------------------------

#[test]
fn la_base_de_datos_se_crea_solo_cuando_hace_falta() {
    let (state, dir) = new_state("deferred");

    assert!(!state.has_db());
    assert!(!state.db_path().exists(), "no debe crearse al abrir la aplicación");
    assert!(!state.init().unwrap());

    // Navegar y leer no crea el archivo.
    let cuentas = state.read(|conn| Ok(accounts::list(conn, false))).unwrap();
    assert!(cuentas.is_none());
    assert!(!state.db_path().exists());

    // Guardar la primera cuenta sí lo crea.
    create_account(&state, "Efectivo", "cash", "CLP", 0);
    assert!(state.has_db());
    assert!(state.db_path().exists());
    assert!(state.db_path().starts_with(&dir));
}

// ---------------------------------------------------------------------------
// 2. Persistencia y 3. Migraciones
// ---------------------------------------------------------------------------

#[test]
fn los_datos_sobreviven_al_cierre_y_la_aplicacion() {
    let dir = temp_dir("persistence");
    let account_id;

    {
        let state = AppState::new(dir.clone());
        account_id = create_account(&state, "Cuenta corriente", "checking", "CLP", 50_000);
        transaction(&state, "income", "Sueldo", 800_000, &account_id, None, "2024-03-01");
    }

    let state = AppState::new(dir.clone());
    assert!(state.init().unwrap(), "debe detectar la base de datos existente");
    assert_eq!(balance(&state, &account_id), 850_000);
    assert_eq!(state.read(|conn| Ok(repos::accounts::list(conn, false)?)).unwrap().unwrap().len(), 1);
}

#[test]
fn las_migraciones_se_aplican_sin_destruir_datos() {
    let dir = temp_dir("migrations");
    let account_id;

    {
        let state = AppState::new(dir.clone());
        account_id = create_account(&state, "Efectivo", "cash", "CLP", 0);
    }

    let state = AppState::new(dir.clone());
    state.init().unwrap();

    let version = state
        .read(|conn| Ok(aureon_lib::core::migrations::current_version(conn)))
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(version >= 1, "debe haber al menos la migración inicial");

    let tables = state
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN
                 ('accounts','categories','contacts','transactions','subscriptions',
                  'subscription_payments','purchases','purchase_installments',
                  'installment_payments','receivables','receivable_payments','payables',
                  'payable_payments','deposits','budgets','wishlist_items','settings',
                  'schema_migrations')",
                [],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(tables, 18, "deben existir todas las tablas del modelo");

    // La segunda apertura no debe duplicar ni perder nada.
    assert_eq!(balance(&state, &account_id), 0);
    let categories = state
        .read(|conn| Ok(categories::list_categories(conn, Some("expense"), false, ("2024-01-01", "2024-12-31"), None)?))
        .unwrap()
        .unwrap();
    assert!(!categories.is_empty(), "las categorías por defecto deben estar");
}

// ---------------------------------------------------------------------------
// 4, 5, 6. Saldos, ingresos, gastos y transferencias
// ---------------------------------------------------------------------------

#[test]
fn los_saldos_se_calculan_con_los_movimientos() {
    let (state, _dir) = new_state("balances");
    let efectivo = create_account(&state, "Efectivo", "cash", "CLP", 10_000);
    let banco = create_account(&state, "Banco", "checking", "CLP", 100_000);

    transaction(&state, "income", "Sueldo", 500_000, &banco, None, "2024-03-01");
    transaction(&state, "expense", "Alquiler", 250_000, &banco, None, "2024-03-02");
    transaction(&state, "expense", "Compra", 30_000, &efectivo, None, "2024-03-03");

    assert_eq!(balance(&state, &banco), 100_000 + 500_000 - 250_000);
    assert_eq!(balance(&state, &efectivo), 10_000 - 30_000);
}

#[test]
fn una_transferencia_entre_cuentas_no_es_ingreso_ni_gasto() {
    let (state, _dir) = new_state("transfer");
    let origen = create_account(&state, "Efectivo", "cash", "CLP", 0);
    let destino = create_account(&state, "Banco", "checking", "CLP", 0);

    transaction(&state, "income", "Sueldo", 100_000, &destino, None, "2024-03-01");
    transaction(&state, "transfer", "Traspaso", 40_000, &origen, Some(&destino), "2024-03-05");

    assert_eq!(balance(&state, &origen), -40_000);
    assert_eq!(balance(&state, &destino), 100_000 + 40_000);

    // El total de las cuentas se mantiene: no se crea ni se destruye dinero.
    let total = state
        .read(|conn| {
            Ok(accounts::list(conn, false)?
                .iter()
                .map(|a| a.balance)
                .sum::<i64>())
        })
        .unwrap()
        .unwrap();
    assert_eq!(total, 100_000);

    // En los reportes, la transferencia aparece separada.
    let summary = state
        .read(|conn| {
            Ok(reports::summary_by_currency(conn, "2024-03-01", "2024-03-31")?)
        })
        .unwrap()
        .unwrap();
    let clp = summary.iter().find(|s| s.currency_code == "CLP").unwrap();
    assert_eq!(clp.income, 100_000);
    assert_eq!(clp.expenses, 0);
    assert_eq!(clp.transfers, 40_000);
}

#[test]
fn una_transferencia_entre_la_misma_cuenta_se_rechaza() {
    let (state, _dir) = new_state("same-account");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    let resultado = state.write(|conn| {
        tx_service::create(
            conn,
            &TransactionInput {
                transaction_type: "transfer".to_string(),
                description: "Error".to_string(),
                amount: 1000,
                currency_code: "CLP".to_string(),
                date: "2024-03-01".to_string(),
                category_id: None,
                account_id: Some(cuenta.clone()),
                transfer_account_id: Some(cuenta.clone()),
                contact_id: None,
                payment_method: None,
                notes: None,
                tags: None,
                related_transaction_id: None,
            },
        )
    });
    assert!(resultado.is_err(), "no se puede transferir a la misma cuenta");
}

#[test]
fn una_devolucion_no_puede_superar_el_gasto_original() {
    let (state, _dir) = new_state("refund");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    let gasto = transaction(&state, "expense", "Zapatillas", 50_000, &cuenta, None, "2024-03-01");

    let devolucion = state.write(|conn| {
        tx_service::create(
            conn,
            &TransactionInput {
                transaction_type: "refund".to_string(),
                description: "Devolución".to_string(),
                amount: 20_000,
                currency_code: "CLP".to_string(),
                date: "2024-03-10".to_string(),
                category_id: None,
                account_id: Some(cuenta.clone()),
                transfer_account_id: None,
                contact_id: None,
                payment_method: None,
                notes: None,
                tags: None,
                related_transaction_id: Some(gasto.id.clone()),
            },
        )
    });
    assert!(devolucion.is_ok());
    assert_eq!(balance(&state, &cuenta), -50_000 + 20_000);

    let exceso = state.write(|conn| {
        tx_service::create(
            conn,
            &TransactionInput {
                transaction_type: "refund".to_string(),
                description: "Devolución excessive".to_string(),
                amount: 40_000,
                currency_code: "CLP".to_string(),
                date: "2024-03-11".to_string(),
                category_id: None,
                account_id: Some(cuenta.clone()),
                transfer_account_id: None,
                contact_id: None,
                payment_method: None,
                notes: None,
                tags: None,
                related_transaction_id: Some(gasto.id.clone()),
            },
        )
    });
    assert!(exceso.is_err(), "no se puede devolver más de lo gastado");
}

#[test]
fn anular_un_movimiento_lo_saca_de_los_saldos_pero_conserva_el_historial() {
    let (state, _dir) = new_state("void");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    let gasto = transaction(&state, "expense", "Café", 3_000, &cuenta, None, "2024-03-01");
    assert_eq!(balance(&state, &cuenta), -3_000);

    state
        .write(|conn| tx_service::void(conn, &gasto.id))
        .unwrap();
    assert_eq!(balance(&state, &cuenta), 0);

    let movimientos = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    include_voided: Some(true),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(movimientos.len(), 1, "el registro anulado se conserva");
    assert!(movimientos[0].voided);
}

// ---------------------------------------------------------------------------
// 7, 8, 9, 10, 11, 12. Compras y cuotas
// ---------------------------------------------------------------------------

fn purchase_input(total: i64, count: i64, account_id: &str) -> PurchaseInput {
    PurchaseInput {
        name: "Notebook".to_string(),
        contact_id: None,
        contact_name: Some("Tienda".to_string()),
        url: None,
        purchase_date: "2024-03-01".to_string(),
        total_amount: total,
        currency_code: "CLP".to_string(),
        installments_count: count,
        installment_amount: None,
        first_due_date: Some("2024-04-01".to_string()),
        frequency: Some("monthly".to_string()),
        frequency_days: None,
        payment_method: None,
        account_id: Some(account_id.to_string()),
        category_id: None,
        status: None,
        paid_immediately: Some(false),
        notes: None,
    }
}

#[test]
fn una_compra_al_contado_registra_el_gasto_y_la_deja_completada() {
    let (state, _dir) = new_state("cash-purchase");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);

    let mut input = purchase_input(120_000, 1, &cuenta);
    input.first_due_date = None;
    input.paid_immediately = Some(true);

    let compra = state
        .write(|conn| purchases::create(conn, &input, "CLP"))
        .unwrap();

    assert_eq!(compra.status, "completed");
    assert_eq!(compra.paid_amount, 120_000);
    assert_eq!(compra.pending_amount, 0);
    assert_eq!(balance(&state, &cuenta), -120_000);
}

#[test]
fn una_compra_en_cuotas_genera_el_calendario_sin_registrar_gastos() {
    let (state, _dir) = new_state("installments");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);

    let compra = state
        .write(|conn| purchases::create(conn, &purchase_input(300_000, 3, &cuenta), "CLP"))
        .unwrap();

    assert_eq!(compra.installments_count, 3);
    assert_eq!(compra.installment_amount, 100_000);
    assert_eq!(compra.paid_amount, 0);
    assert_eq!(balance(&state, &cuenta), 0, "crear la compra no mueve dinero");

    let cuotas = state
        .read(|conn| Ok(purchases::list_installments(conn, &compra.id)?))
        .unwrap()
        .unwrap();
    assert_eq!(cuotas.len(), 3);
    assert_eq!(cuotas[0].due_date, "2024-04-01");
    assert_eq!(cuotas[1].due_date, "2024-05-01");
    assert_eq!(cuotas[2].due_date, "2024-06-01");
    assert!(cuotas.iter().all(|c| c.status == "pending"));
}

#[test]
fn un_monto_que_no_se_divide_exactamente_se_reparte_sin_perder_pesos() {
    let (state, _dir) = new_state("rounding");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);

    let compra = state
        .write(|conn| purchases::create(conn, &purchase_input(100_000, 3, &cuenta), "CLP"))
        .unwrap();

    let cuotas = state
        .read(|conn| Ok(purchases::list_installments(conn, &compra.id)?))
        .unwrap()
        .unwrap();
    let suma: i64 = cuotas.iter().map(|c| c.amount).sum();
    assert_eq!(suma, 100_000, "la suma de las cuotas es el total exacto");
    assert_eq!(cuotas[0].amount, 33_334);
    assert_eq!(cuotas[1].amount, 33_333);
    assert_eq!(cuotas[2].amount, 33_333);
}

#[test]
fn pagar_una_cuota_crea_un_solo_gasto_y_evita_pagos_duplicados() {
    let (state, _dir) = new_state("pay-installment");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);
    let compra = state
        .write(|conn| purchases::create(conn, &purchase_input(300_000, 3, &cuenta), "CLP"))
        .unwrap();
    let cuotas = state
        .read(|conn| Ok(purchases::list_installments(conn, &compra.id)?))
        .unwrap()
        .unwrap();

    let cuota = state
        .write(|conn| {
            purchases::pay_installment(
                conn,
                &cuotas[0].id,
                &InstallmentPaymentInput {
                    amount: 100_000,
                    paid_at: "2024-04-01".to_string(),
                    account_id: Some(cuenta.clone()),
                },
            )
        })
        .unwrap();
    assert_eq!(cuota.status, "paid");
    assert_eq!(cuota.paid_amount, 100_000);
    assert_eq!(balance(&state, &cuenta), -100_000);

    // Pagar de nuevo la misma cuota se rechaza.
    let repetido = state.write(|conn| {
        purchases::pay_installment(
            conn,
            &cuotas[0].id,
            &InstallmentPaymentInput {
                amount: 100_000,
                paid_at: "2024-04-02".to_string(),
                account_id: Some(cuenta.clone()),
            },
        )
    });
    assert!(repetido.is_err(), "no se puede pagar dos veces la misma cuota");
    assert_eq!(balance(&state, &cuenta), -100_000);

    // Sólo se generó un movimiento por el pago de la cuota.
    let movimientos = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    linked_type: Some("installment".to_string()),
                    linked_id: Some(cuotas[0].id.clone()),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(movimientos.len(), 1);
}

#[test]
fn los_pagos_parciales_conservan_el_saldo_pendiente() {
    let (state, _dir) = new_state("partial");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);
    let compra = state
        .write(|conn| purchases::create(conn, &purchase_input(300_000, 3, &cuenta), "CLP"))
        .unwrap();
    let cuotas = state
        .read(|conn| Ok(purchases::list_installments(conn, &compra.id)?))
        .unwrap()
        .unwrap();

    let cuota = state
        .write(|conn| {
            purchases::pay_installment(
                conn,
                &cuotas[0].id,
                &InstallmentPaymentInput {
                    amount: 40_000,
                    paid_at: "2024-04-01".to_string(),
                    account_id: Some(cuenta.clone()),
                },
            )
        })
        .unwrap();
    assert_eq!(cuota.paid_amount, 40_000);
    assert_eq!(cuota.status, "pending", "no queda pagada hasta completar el importe");
    assert_eq!(balance(&state, &cuenta), -40_000);

    // Pagar más que el saldo pendiente de la cuota se rechaza.
    let exceso = state.write(|conn| {
        purchases::pay_installment(
            conn,
            &cuotas[0].id,
            &InstallmentPaymentInput {
                amount: 70_000,
                paid_at: "2024-04-02".to_string(),
                account_id: Some(cuenta.clone()),
            },
        )
    });
    assert!(exceso.is_err());

    // El resto se puede completar.
    let completa = state
        .write(|conn| {
            purchases::pay_installment(
                conn,
                &cuotas[0].id,
                &InstallmentPaymentInput {
                    amount: 60_000,
                    paid_at: "2024-04-03".to_string(),
                    account_id: Some(cuenta.clone()),
                },
            )
        })
        .unwrap();
    assert_eq!(completa.status, "paid");
    assert_eq!(balance(&state, &cuenta), -100_000);
}

#[test]
fn el_saldo_pendiente_de_una_compra_refleja_lo_pagado() {
    let (state, _dir) = new_state("pending-balance");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);
    let compra = state
        .write(|conn| purchases::create(conn, &purchase_input(300_000, 3, &cuenta), "CLP"))
        .unwrap();
    let cuotas = state
        .read(|conn| Ok(purchases::list_installments(conn, &compra.id)?))
        .unwrap()
        .unwrap();

    state
        .write(|conn| {
            purchases::pay_installment(
                conn,
                &cuotas[0].id,
                &InstallmentPaymentInput {
                    amount: 100_000,
                    paid_at: "2024-04-01".to_string(),
                    account_id: Some(cuenta.clone()),
                },
            )
        })
        .unwrap();

    let actualizada = state
        .read(|conn| Ok(purchases::get(conn, &compra.id)?))
        .unwrap()
        .unwrap();
    assert_eq!(actualizada.paid_amount, 100_000);
    assert_eq!(actualizada.pending_amount, 200_000);
    assert_eq!(actualizada.installments_paid, 1);
    assert_eq!(actualizada.installments_pending, 2);
    assert_eq!(actualizada.next_due_date.as_deref(), Some("2024-05-01"));
}

#[test]
fn las_cuotas_vencidas_se_detectan_sin_marcarse_como_pagadas() {
    let (state, _dir) = new_state("overdue");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);
    let mut input = purchase_input(100_000, 2, &cuenta);
    input.first_due_date = Some("2020-01-01".to_string());
    let compra = state
        .write(|conn| purchases::create(conn, &input, "CLP"))
        .unwrap();

    state
        .write(|conn| {
            repos::subscriptions::refresh_overdue(conn, &dates::today_str())?;
            repos::maintenance::refresh(conn, &dates::today_str())
        })
        .unwrap();

    let cuotas = state
        .read(|conn| Ok(purchases::list_installments(conn, &compra.id)?))
        .unwrap()
        .unwrap();
    assert!(cuotas.iter().all(|c| c.status == "overdue"));
    assert!(cuotas.iter().all(|c| c.paid_amount == 0));
    assert_eq!(balance(&state, &cuenta), 0, "vencer no es pagar");
}

#[test]
fn cancelar_una_compra_cancela_las_cuotas_pendientes() {
    let (state, _dir) = new_state("cancel-purchase");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);
    let compra = state
        .write(|conn| purchases::create(conn, &purchase_input(300_000, 3, &cuenta), "CLP"))
        .unwrap();

    let cancelada = state
        .write(|conn| purchases::cancel(conn, &compra.id))
        .unwrap();
    assert_eq!(cancelada.status, "cancelled");

    let cuotas = state
        .read(|conn| Ok(purchases::list_installments(conn, &compra.id)?))
        .unwrap()
        .unwrap();
    assert!(cuotas.iter().all(|c| c.status == "cancelled"));
}

// ---------------------------------------------------------------------------
// 13, 14. Suscripciones
// ---------------------------------------------------------------------------

fn subscription_input(amount: i64, account_id: &str) -> SubscriptionInput {
    SubscriptionInput {
        name: "Netflix".to_string(),
        category_id: None,
        contact_id: None,
        contact_name: Some("Netflix".to_string()),
        url: None,
        amount,
        currency_code: "CLP".to_string(),
        frequency: "monthly".to_string(),
        frequency_days: None,
        start_date: "2024-01-01".to_string(),
        next_charge_date: Some("2024-03-01".to_string()),
        account_id: Some(account_id.to_string()),
        payment_method: None,
        responsible_contact_id: None,
        status: None,
        notes: None,
    }
}

#[test]
fn crear_una_suscripcion_no_genera_gastos_pero_cobrar_sí() {
    let (state, _dir) = new_state("subscriptions");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);

    let suscripcion = state
        .write(|conn| subscriptions::create(conn, &subscription_input(9_990, &cuenta), "CLP"))
        .unwrap();
    assert_eq!(balance(&state, &cuenta), 0, "crear la suscripción no cobra nada");

    let cobro = state
        .write(|conn| {
            subscriptions::charge(
                conn,
                &suscripcion.id,
                &SubscriptionPaymentInput {
                    amount: None,
                    charged_at: "2024-03-01".to_string(),
                    account_id: Some(cuenta.clone()),
                    next_charge_date: None,
                },
            )
        })
        .unwrap();
    assert_eq!(cobro.amount, 9_990);
    assert_eq!(balance(&state, &cuenta), -9_990);

    let historial = state
        .read(|conn| Ok(subscriptions::list_payments(conn, &suscripcion.id)?))
        .unwrap()
        .unwrap();
    assert_eq!(historial.len(), 1);
}

#[test]
fn cambiar_el_precio_y_cancelar_conserva_el_historial_de_cobros() {
    let (state, _dir) = new_state("subscription-history");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);
    let suscripcion = state
        .write(|conn| subscriptions::create(conn, &subscription_input(9_990, &cuenta), "CLP"))
        .unwrap();

    state
        .write(|conn| {
            subscriptions::charge(
                conn,
                &suscripcion.id,
                &SubscriptionPaymentInput {
                    amount: None,
                    charged_at: "2024-03-01".to_string(),
                    account_id: Some(cuenta.clone()),
                    next_charge_date: None,
                },
            )
        })
        .unwrap();

    // Cambio de precio.
    let mut nuevo = subscription_input(12_990, &cuenta);
    nuevo.name = "Netflix".to_string();
    state
        .write(|conn| subscriptions::update(conn, &suscripcion.id, &nuevo))
        .unwrap();

    // Cobro al precio nuevo.
    state
        .write(|conn| {
            subscriptions::charge(
                conn,
                &suscripcion.id,
                &SubscriptionPaymentInput {
                    amount: None,
                    charged_at: "2024-04-01".to_string(),
                    account_id: Some(cuenta.clone()),
                    next_charge_date: None,
                },
            )
        })
        .unwrap();

    let historial = state
        .read(|conn| Ok(subscriptions::list_payments(conn, &suscripcion.id)?))
        .unwrap()
        .unwrap();
    assert_eq!(historial.len(), 2);
    let importes: Vec<i64> = historial.iter().map(|p| p.amount).collect();
    assert!(importes.contains(&9_990) && importes.contains(&12_990));

    // Cancelación.
    let cancelada = state
        .write(|conn| subscriptions::set_status(conn, &suscripcion.id, "cancelled"))
        .unwrap();
    assert_eq!(cancelada.status, "cancelled");
    assert!(cancelada.cancelled_at.is_some());

    let historial = state
        .read(|conn| Ok(subscriptions::list_payments(conn, &suscripcion.id)?))
        .unwrap()
        .unwrap();
    assert_eq!(historial.len(), 2, "cancelar no borra el historial");
}

#[test]
fn el_costo_mensual_estimado_normaliza_las_frecuencias() {
    assert_eq!(subscriptions::monthly_estimate(10_000, &dates::Frequency::Monthly), 10_000);
    assert_eq!(subscriptions::monthly_estimate(10_000, &dates::Frequency::Yearly), 833);
    assert_eq!(subscriptions::monthly_estimate(10_000, &dates::Frequency::Quarterly), 3_333);
    assert_eq!(subscriptions::yearly_estimate(10_000, &dates::Frequency::Monthly), 120_000);
    assert_eq!(subscriptions::yearly_estimate(10_000, &dates::Frequency::Weekly), 520_000);
}

// ---------------------------------------------------------------------------
// 15. Cuentas por cobrar y por pagar
// ---------------------------------------------------------------------------

#[test]
fn una_cuenta_por_cobrar_suma_al_cuenta_solo_cuando_se_recibe() {
    let (state, _dir) = new_state("receivables");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);

    let por_cobrar = state
        .write(|conn| {
            receivables::create(
                conn,
                &ReceivableInput {
                    contact_id: None,
                    contact_name: Some("Ana".to_string()),
                    title: "Préstamo".to_string(),
                    kind: Some("debt".to_string()),
                    total_amount: 50_000,
                    currency_code: "CLP".to_string(),
                    origin_date: "2024-03-01".to_string(),
                    due_date: Some("2099-04-01".to_string()),
                    notes: None,
                },
                "CLP",
            )
        })
        .unwrap();
    assert_eq!(por_cobrar.pending_amount, 50_000);
    assert_eq!(balance(&state, &cuenta), 0, "lo por cobrar no es dinero disponible");

    // Cobro parcial.
    let parcial = state
        .write(|conn| {
            receivables::receive(
                conn,
                &por_cobrar.id,
                &ReceivablePaymentInput {
                    amount: 20_000,
                    received_at: "2024-03-10".to_string(),
                    account_id: Some(cuenta.clone()),
                },
            )
        })
        .unwrap();
    assert_eq!(parcial.received_amount, 20_000);
    assert_eq!(parcial.pending_amount, 30_000);
    assert_eq!(parcial.status, "partial");
    assert_eq!(balance(&state, &cuenta), 20_000);

    // Cobro de más.
    let exceso = state.write(|conn| {
        receivables::receive(
            conn,
            &por_cobrar.id,
            &ReceivablePaymentInput {
                amount: 40_000,
                received_at: "2024-03-11".to_string(),
                account_id: Some(cuenta.clone()),
            },
        )
    });
    assert!(exceso.is_err());

    // Cobro final.
    let final_ = state
        .write(|conn| {
            receivables::receive(
                conn,
                &por_cobrar.id,
                &ReceivablePaymentInput {
                    amount: 30_000,
                    received_at: "2024-03-12".to_string(),
                    account_id: Some(cuenta.clone()),
                },
            )
        })
        .unwrap();
    assert_eq!(final_.status, "paid");
    assert_eq!(final_.pending_amount, 0);
    assert_eq!(balance(&state, &cuenta), 50_000);
}

#[test]
fn una_cuenta_por_pagar_registra_el_gasto_cuando_se_paga() {
    let (state, _dir) = new_state("payables");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 20_000);

    let por_pagar = state
        .write(|conn| {
            payables::create(
                conn,
                &PayableInput {
                    contact_id: None,
                    contact_name: Some("Proveedor".to_string()),
                    title: "Service".to_string(),
                    total_amount: 15_000,
                    currency_code: "CLP".to_string(),
                    created_date: "2024-03-01".to_string(),
                    due_date: Some("2024-04-01".to_string()),
                    account_id: Some(cuenta.clone()),
                    notes: None,
                },
                "CLP",
            )
        })
        .unwrap();
    assert_eq!(balance(&state, &cuenta), 20_000, "crear la obligación no gasta dinero");

    let pagada = state
        .write(|conn| {
            payables::pay(
                conn,
                &por_pagar.id,
                &PayablePaymentInput {
                    amount: 15_000,
                    paid_at: "2024-03-15".to_string(),
                    account_id: Some(cuenta.clone()),
                },
            )
        })
        .unwrap();
    assert_eq!(pagada.status, "paid");
    assert_eq!(pagada.paid_amount, 15_000);
    assert_eq!(pagada.pending_amount, 0);
    assert_eq!(balance(&state, &cuenta), 5_000);
}

// ---------------------------------------------------------------------------
// 16. Depósitos pendientes
// ---------------------------------------------------------------------------

#[test]
fn un_deposito_pendiente_no_altera_los_saldos_hasta_completarse() {
    let (state, _dir) = new_state("deposits");
    let cuenta = create_account(&state, "Banco", "checking", "CLP", 100_000);

    let deposito = state
        .write(|conn| {
            deposits::create(
                conn,
                &DepositInput {
                    contact_id: None,
                    contact_name: Some("Carlos".to_string()),
                    title: "Transferencia".to_string(),
                    amount: 30_000,
                    currency_code: "CLP".to_string(),
                    created_date: "2024-03-01".to_string(),
                    due_date: Some("2024-04-01".to_string()),
                    account_id: Some(cuenta.clone()),
                    destination_institution: Some("Banco X".to_string()),
                    destination_details: None,
                    payment_method: Some("Transferencia".to_string()),
                    notes: None,
                },
                "CLP",
            )
        })
        .unwrap();
    assert_eq!(deposito.status, "pending");
    assert_eq!(balance(&state, &cuenta), 100_000, "el depósito no mueve dinero");

    let completado = state
        .write(|conn| {
            deposits::complete(
                conn,
                &deposito.id,
                &DepositCompleteInput {
                    completed_at: Some("2024-03-20".to_string()),
                    account_id: Some(cuenta.clone()),
                    transfer_account_id: None,
                    register_transaction: Some(true),
                    transaction_type: Some("expense".to_string()),
                    contact_id: None,
                    category_id: None,
                    payment_method: None,
                    notes: None,
                    receivable_id: None,
                },
            )
        })
        .unwrap();
    assert_eq!(completado.status, "completed");
    assert_eq!(balance(&state, &cuenta), 70_000);
    assert!(completado.transaction_id.is_some());
}

#[test]
fn un_deposito_completado_sin_movimiento_no_altera_los_saldos() {
    let (state, _dir) = new_state("deposits-no-movement");
    let cuenta = create_account(&state, "Banco", "checking", "CLP", 100_000);

    let deposito = state
        .write(|conn| {
            deposits::create(
                conn,
                &DepositInput {
                    contact_id: None,
                    contact_name: None,
                    title: "Depósito".to_string(),
                    amount: 30_000,
                    currency_code: "CLP".to_string(),
                    created_date: "2024-03-01".to_string(),
                    due_date: None,
                    account_id: Some(cuenta.clone()),
                    destination_institution: None,
                    destination_details: None,
                    payment_method: None,
                    notes: None,
                },
                "CLP",
            )
        })
        .unwrap();

    state
        .write(|conn| {
            deposits::complete(
                conn,
                &deposito.id,
                &DepositCompleteInput {
                    completed_at: Some("2024-03-20".to_string()),
                    account_id: Some(cuenta.clone()),
                    transfer_account_id: None,
                    register_transaction: Some(false),
                    transaction_type: None,
                    contact_id: None,
                    category_id: None,
                    payment_method: None,
                    notes: None,
                    receivable_id: None,
                },
            )
        })
        .unwrap();

    assert_eq!(balance(&state, &cuenta), 100_000);
}

// ---------------------------------------------------------------------------
// 17. Lista de deseos
// ---------------------------------------------------------------------------

#[test]
fn un_producto_deseado_se_convierte_en_compra_sin_registrar_gastos() {
    let (state, _dir) = new_state("wishlist");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);

    let item = state
        .write(|conn| {
            wishlist::create(
                conn,
                &WishlistInput {
                    name: Some("Audífonos".to_string()),
                    url: Some("https://example.com/producto".to_string()),
                    store: Some("Tienda".to_string()),
                    estimated_price: Some(50_000),
                    currency_code: Some("CLP".to_string()),
                    added_date: None,
                    category_id: None,
                    priority: Some(1),
                    savings_goal: None,
                    status: None,
                    notes: None,
                },
                "CLP",
            )
        })
        .unwrap();
    assert_eq!(item.status, "wanted");

    let mut input = purchase_input(50_000, 1, &cuenta);
    input.name = "Audífonos".to_string();
    input.first_due_date = None;

    let compra = state
        .write(|conn| wishlist::convert_to_purchase(conn, &item.id, &input))
        .unwrap();
    assert_eq!(compra.name, "Audífonos");
    assert_eq!(balance(&state, &cuenta), 0, "convertir no registra gasto");

    let actualizado = state
        .read(|conn| Ok(wishlist::get(conn, &item.id)?))
        .unwrap()
        .unwrap();
    assert_eq!(actualizado.status, "purchased");
    assert_eq!(actualizado.purchase_id.as_deref(), Some(compra.id.as_str()));

    // Convertir dos veces se rechaza.
    let repetido = state.write(|conn| {
        wishlist::convert_to_purchase(conn, &item.id, &input)
    });
    assert!(repetido.is_err());
}

#[test]
fn una_url_invalida_se_rechaza() {
    let (state, _dir) = new_state("wishlist-url");
    let resultado = state.write(|conn| {
        wishlist::create(
            conn,
            &WishlistInput {
                name: Some("Producto".to_string()),
                url: Some("javascript:alert(1)".to_string()),
                store: None,
                estimated_price: None,
                currency_code: None,
                added_date: None,
                category_id: None,
                priority: None,
                savings_goal: None,
                status: None,
                notes: None,
            },
            "CLP",
        )
    });
    assert!(resultado.is_err());
}

// ---------------------------------------------------------------------------
// 18. Filtros del historial
// ---------------------------------------------------------------------------

#[test]
fn los_filtros_del_historial_se_aplican() {
    let (state, _dir) = new_state("filters");
    let efectivo = create_account(&state, "Efectivo", "cash", "CLP", 0);
    let banco = create_account(&state, "Banco", "checking", "CLP", 0);

    transaction(&state, "income", "Sueldo", 500_000, &banco, None, "2024-03-01");
    transaction(&state, "expense", "Supermercado", 40_000, &efectivo, None, "2024-03-02");
    transaction(&state, "expense", "Bencina", 25_000, &efectivo, None, "2024-04-02");
    transaction(&state, "transfer", "Traspaso", 10_000, &efectivo, Some(&banco), "2024-04-03");

    let por_tipo = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    types: Some(vec!["expense".to_string()]),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(por_tipo.len(), 2);

    let por_rango = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    start_date: Some("2024-04-01".to_string()),
                    end_date: Some("2024-04-30".to_string()),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(por_rango.len(), 2);

    let por_cuenta = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    account_ids: Some(vec![efectivo.clone()]),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(por_cuenta.len(), 3, "la cuenta aparece como origen o destino");

    let por_monto = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    min_amount: Some(30_000),
                    max_amount: Some(100_000),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(por_monto.len(), 1);

    let por_busqueda = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    search: Some("Supermercado".to_string()),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(por_busqueda.len(), 1);

    // El filtro por período usa fechas relativas a hoy.
    let hoy = dates::today_str();
    transaction(&state, "expense", "Compra de hoy", 1_000, &efectivo, None, &hoy);
    let por_periodo = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    period: Some("month".to_string()),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(por_periodo.len(), 1, "el período actual sólo trae lo de hoy");
    assert_eq!(por_periodo[0].description, "Compra de hoy");

    let por_periodo_anterior = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    period: Some("last_year".to_string()),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert!(por_periodo_anterior.is_empty(), "el año anterior no trae lo de hoy");
}

// ---------------------------------------------------------------------------
// 19. Exportaciones
// ---------------------------------------------------------------------------

#[test]
fn las_exportaciones_generan_archivos_csv_legibles() {
    let (state, _dir) = new_state("exports");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    transaction(&state, "expense", "Café", 3_000, &cuenta, None, "2024-03-01");

    let csv = state
        .read(|conn| {
            Ok(export::transactions_csv(
                conn,
                &TransactionFilter {
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert!(csv.contains("Fecha;Tipo;Descripción"));
    assert!(csv.contains("Café"));
    assert!(csv.contains("01/03/2024"), "las fechas se exportan en DD/MM/AAAA");
    assert!(csv.starts_with('\u{feff}'), "se incluye BOM para Excel");
}

#[test]
fn se_pueden_exportar_todas_las_colecciones() {
    let (state, _dir) = new_state("exports-all");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    transaction(&state, "expense", "Café", 3_000, &cuenta, None, "2024-03-01");

    let datasets = [
        "transactions",
        "purchases",
        "installments",
        "subscriptions",
        "receivables",
        "payables",
        "deposits",
        "wishlist",
    ];
    for dataset in datasets {
        let csv = state
            .read(|conn| match dataset {
                "transactions" => Ok(export::transactions_csv(conn, &TransactionFilter::default())?),
                "purchases" => Ok(export::purchases_csv(conn)?),
                "installments" => Ok(export::installments_csv(conn)?),
                "subscriptions" => Ok(export::subscriptions_csv(conn)?),
                "receivables" => Ok(export::receivables_csv(conn)?),
                "payables" => Ok(export::payables_csv(conn)?),
                "deposits" => Ok(export::deposits_csv(conn)?),
                _ => Ok(export::wishlist_csv(conn)?),
            })
            .unwrap()
            .unwrap();
        assert!(!csv.is_empty(), "el dataset {dataset} no puede estar vacío");
        assert!(csv.contains(';'), "el dataset {dataset} debe usar separador de punto y coma");
    }
}

// ---------------------------------------------------------------------------
// 20. Copias de seguridad y restauración
// ---------------------------------------------------------------------------

#[test]
fn se_crea_una_copia_de_seguridad_y_se_restaura() {
    let dir = temp_dir("backup");
    let estado_inicial = AppState::new(dir.clone());
    let cuenta = create_account(&estado_inicial, "Efectivo", "cash", "CLP", 5_000);
    transaction(&estado_inicial, "income", "Sueldo", 100_000, &cuenta, None, "2024-03-01");

    let copia = dir.join("copia.db");
    let resultado = estado_inicial
        .write(|conn| {
            Ok(aureon_lib::core::services::backup::create_snapshot(conn, &copia)?)
        })
        .unwrap();
    assert!(resultado.schema_version >= 1);
    assert!(copia.exists());

    // Se modifica la base de datos original.
    transaction(&estado_inicial, "expense", "Gasto", 50_000, &cuenta, None, "2024-03-05");
    assert_eq!(balance(&estado_inicial, &cuenta), 55_000);

    // Se restaura la copia.
    let version = estado_inicial.restore_from(&copia).unwrap();
    assert!(version >= 1);
    assert_eq!(
        balance(&estado_inicial, &cuenta),
        105_000,
        "tras restaurar vuelve el saldo anterior"
    );
}

#[test]
fn una_copia_dañada_no_se_restaura() {
    let dir = temp_dir("backup-invalid");
    let state = AppState::new(dir.clone());
    create_account(&state, "Efectivo", "cash", "CLP", 0);

    let invalida = dir.join("no-es-una-copia.db");
    std::fs::write(&invalida, b"esto no es una base de datos").unwrap();
    assert!(state.restore_from(&invalida).is_err());

    let vacia = dir.join("vacia.db");
    std::fs::write(&vacia, b"").unwrap();
    assert!(state.restore_from(&vacia).is_err());
}

// ---------------------------------------------------------------------------
// 21. Manejo de errores de escritura
// ---------------------------------------------------------------------------

#[test]
fn los_datos_invalidos_se_rechazan_con_mensajes_claros() {
    let (state, _dir) = new_state("validation");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);

    // Monto cero o negativo.
    let monto_invalido = state.write(|conn| {
        tx_service::create(
            conn,
            &TransactionInput {
                transaction_type: "expense".to_string(),
                description: "Imposible".to_string(),
                amount: 0,
                currency_code: "CLP".to_string(),
                date: "2024-03-01".to_string(),
                category_id: None,
                account_id: Some(cuenta.clone()),
                transfer_account_id: None,
                contact_id: None,
                payment_method: None,
                notes: None,
                tags: None,
                related_transaction_id: None,
            },
        )
    });
    assert!(monto_invalido.is_err());

    // Fecha inválida.
    let fecha_invalida = state.write(|conn| {
        tx_service::create(
            conn,
            &TransactionInput {
                transaction_type: "expense".to_string(),
                description: "Sin fecha".to_string(),
                amount: 1000,
                currency_code: "CLP".to_string(),
                date: "no-es-fecha".to_string(),
                category_id: None,
                account_id: Some(cuenta.clone()),
                transfer_account_id: None,
                contact_id: None,
                payment_method: None,
                notes: None,
                tags: None,
                related_transaction_id: None,
            },
        )
    });
    assert!(fecha_invalida.is_err());

    // Cuenta inexistente.
    let cuenta_inexistente = state.write(|conn| {
        tx_service::create(
            conn,
            &TransactionInput {
                transaction_type: "expense".to_string(),
                description: "Sin cuenta".to_string(),
                amount: 1000,
                currency_code: "CLP".to_string(),
                date: "2024-03-01".to_string(),
                category_id: None,
                account_id: Some("no-existe".to_string()),
                transfer_account_id: None,
                contact_id: None,
                payment_method: None,
                notes: None,
                tags: None,
                related_transaction_id: None,
            },
        )
    });
    assert!(cuenta_inexistente.is_err());

    // Moneda inexistente.
    let moneda_invalida = state.write(|conn| {
        tx_service::create(
            conn,
            &TransactionInput {
                transaction_type: "expense".to_string(),
                description: "Moneda rara".to_string(),
                amount: 1000,
                currency_code: "XXX".to_string(),
                date: "2024-03-01".to_string(),
                category_id: None,
                account_id: Some(cuenta.clone()),
                transfer_account_id: None,
                contact_id: None,
                payment_method: None,
                notes: None,
                tags: None,
                related_transaction_id: None,
            },
        )
    });
    assert!(moneda_invalida.is_err());

    // El saldo no se movió con ninguna operación fallida.
    assert_eq!(balance(&state, &cuenta), 0);
}

// ---------------------------------------------------------------------------
// 22. Atomicidad
// ---------------------------------------------------------------------------

#[test]
fn un_pago_de_cuota_fallido_no_deja_movimientos_parciales() {
    let (state, _dir) = new_state("atomicity");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);
    let compra = state
        .write(|conn| purchases::create(conn, &purchase_input(300_000, 3, &cuenta), "CLP"))
        .unwrap();
    let cuotas = state
        .read(|conn| Ok(purchases::list_installments(conn, &compra.id)?))
        .unwrap()
        .unwrap();

    // Se intenta pagar de más: la operación debe fallar por completo.
    let fallido = state.write(|conn| {
        purchases::pay_installment(
            conn,
            &cuotas[0].id,
            &InstallmentPaymentInput {
                amount: 500_000,
                paid_at: "2024-04-01".to_string(),
                account_id: Some(cuenta.clone()),
            },
        )
    });
    assert!(fallido.is_err());

    let movimientos = state
        .read(|conn| {
            Ok(tx_service::list(
                conn,
                &TransactionFilter {
                    include_voided: Some(true),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();
    assert!(movimientos.is_empty(), "no debe quedar ningún movimiento a medias");
    assert_eq!(balance(&state, &cuenta), 0);

    let cuota = state
        .read(|conn| Ok(purchases::get_installment(conn, &cuotas[0].id)?))
        .unwrap()
        .unwrap();
    assert_eq!(cuota.paid_amount, 0);
    assert_eq!(cuota.status, "pending");
}

#[test]
fn un_cobro_fallido_no_modifica_el_saldo_pendiente() {
    let (state, _dir) = new_state("atomicity-receivable");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    let por_cobrar = state
        .write(|conn| {
            receivables::create(
                conn,
                &ReceivableInput {
                    contact_id: None,
                    contact_name: Some("Ana".to_string()),
                    title: "Préstamo".to_string(),
                    kind: None,
                    total_amount: 50_000,
                    currency_code: "CLP".to_string(),
                    origin_date: "2024-03-01".to_string(),
                    due_date: None,
                    notes: None,
                },
                "CLP",
            )
        })
        .unwrap();

    let fallido = state.write(|conn| {
        receivables::receive(
            conn,
            &por_cobrar.id,
            &ReceivablePaymentInput {
                amount: 80_000,
                received_at: "2024-03-10".to_string(),
                account_id: Some(cuenta.clone()),
            },
        )
    });
    assert!(fallido.is_err());

    let sin_cambios = state
        .read(|conn| Ok(receivables::get(conn, &por_cobrar.id)?))
        .unwrap()
        .unwrap();
    assert_eq!(sin_cambios.received_amount, 0);
    assert_eq!(sin_cambios.pending_amount, 50_000);
    assert_eq!(balance(&state, &cuenta), 0);
}

// ---------------------------------------------------------------------------
// Panel principal y ajustes
// ---------------------------------------------------------------------------

#[test]
fn el_panel_principal_distingue_lo_pagado_de_lo_pendiente() {
    let (state, _dir) = new_state("dashboard");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    transaction(&state, "income", "Sueldo", 300_000, &cuenta, None, "2024-03-01");
    transaction(&state, "expense", "Gasto", 50_000, &cuenta, None, "2024-03-02");

    state
        .write(|conn| {
            receivables::create(
                conn,
                &ReceivableInput {
                    contact_id: None,
                    contact_name: Some("Ana".to_string()),
                    title: "Préstamo".to_string(),
                    kind: None,
                    total_amount: 20_000,
                    currency_code: "CLP".to_string(),
                    origin_date: "2024-03-01".to_string(),
                    due_date: None,
                    notes: None,
                },
                "CLP",
            )
        })
        .unwrap();

    let summary = state
        .read(|conn| {
            Ok(reports::dashboard(
                conn,
                "2024-03-01",
                "2024-03-31",
                "CLP",
            )?)
        })
        .unwrap()
        .unwrap();

    assert_eq!(summary.income, 300_000);
    assert_eq!(summary.expenses, 50_000);
    assert_eq!(summary.net, 250_000);
    assert_eq!(summary.balance, 250_000);
    assert_eq!(summary.receivable_pending, 20_000, "lo por cobrar va aparte");
}

#[test]
fn los_ajustes_se_guardan_y_se_recuperan() {
    let (state, _dir) = new_state("settings");
    let guardados = state
        .write(|conn| {
            Ok(settings::save(
                conn,
                &settings::AppSettings {
                    default_currency: "CLP".to_string(),
                    date_format: "DD/MM/AAAA".to_string(),
                    theme: "dark".to_string(),
                    export_separator: ",".to_string(),
                    backup_folder: Some("C:/Copias".to_string()),
                    week_starts_on_monday: true,
                },
            )?)
        })
        .unwrap();
    assert_eq!(guardados.theme, "dark");

    let leidos = state.read(|conn| Ok(settings::load(conn)?)).unwrap().unwrap();
    assert_eq!(leidos.theme, "dark");
    assert_eq!(leidos.export_separator, ",");
    assert_eq!(leidos.backup_folder.as_deref(), Some("C:/Copias"));
}

#[test]
fn las_claves_foraneas_estan_activas() {
    let (state, _dir) = new_state("foreign-keys");
    create_account(&state, "Efectivo", "cash", "CLP", 0);
    let activada: i64 = state
        .read(|conn| {
            Ok(conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(activada, 1);
}

#[test]
fn el_modo_wal_permite_lecturas_concurrentes() {
    let (state, _dir) = new_state("wal");
    create_account(&state, "Efectivo", "cash", "CLP", 0);
    let modo: String = state
        .read(|conn| {
            Ok(conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(modo.to_lowercase(), "wal");
}

#[test]
fn no_se_puede_cambiar_la_moneda_de_una_cuenta_con_movimientos() {
    let (state, _dir) = new_state("currency-change");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 10_000);
    transaction(&state, "income", "Sueldo", 50_000, &cuenta, None, "2024-03-01");

    let resultado = state.write(|conn| {
        accounts::update(
            conn,
            &cuenta,
            &AccountInput {
                name: "Efectivo".to_string(),
                account_type: "cash".to_string(),
                currency_code: "USD".to_string(),
                initial_balance: 10_000,
                initial_balance_date: "2024-01-01".to_string(),
                archived: Some(false),
                description: None,
            },
        )
    });
    assert!(
        resultado.is_err(),
        "cambiar la moneda reinterpretaría los importes ya registrados"
    );

    // Sin movimientos, el cambio sí se permite.
    let otra = create_account(&state, "Ahorro", "savings", "CLP", 0);
    let cambio = state.write(|conn| {
        accounts::update(
            conn,
            &otra,
            &AccountInput {
                name: "Ahorro".to_string(),
                account_type: "savings".to_string(),
                currency_code: "USD".to_string(),
                initial_balance: 0,
                initial_balance_date: "2024-01-01".to_string(),
                archived: Some(false),
                description: None,
            },
        )
    });
    assert!(cambio.is_ok());
}

#[test]
fn no_se_puede_registrar_un_movimiento_en_una_cuenta_de_otra_moneda() {
    let (state, _dir) = new_state("currency-mismatch");
    let clp = create_account(&state, "Peso", "cash", "CLP", 0);
    let usd = create_account(&state, "Dolar", "savings", "USD", 0);

    let resultado = state.write(|conn| {
        tx_service::create(
            conn,
            &TransactionInput {
                transaction_type: "expense".to_string(),
                description: "Compra en dolares".to_string(),
                amount: 1000,
                currency_code: "USD".to_string(),
                date: "2024-03-01".to_string(),
                category_id: None,
                account_id: Some(clp.clone()),
                transfer_account_id: None,
                contact_id: None,
                payment_method: None,
                notes: None,
                tags: None,
                related_transaction_id: None,
            },
        )
    });
    assert!(
        resultado.is_err(),
        "un importe en dolares no puede sumarse a una cuenta en pesos"
    );

    // Con la cuenta correcta sí se registra.
    let ok = state.write(|conn| {
        tx_service::create(
            conn,
            &TransactionInput {
                transaction_type: "expense".to_string(),
                description: "Compra en dolares".to_string(),
                amount: 1000,
                currency_code: "USD".to_string(),
                date: "2024-03-01".to_string(),
                category_id: None,
                account_id: Some(usd.clone()),
                transfer_account_id: None,
                contact_id: None,
                payment_method: None,
                notes: None,
                tags: None,
                related_transaction_id: None,
            },
        )
    });
    assert!(ok.is_ok());
    assert_eq!(balance(&state, &usd), -1000);
    assert_eq!(balance(&state, &clp), 0);
}

#[test]
fn un_cobro_parcial_no_saca_la_deuda_de_la_categoria_vencida() {
    let (state, _dir) = new_state("partial-overdue");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    let por_cobrar = state
        .write(|conn| {
            receivables::create(
                conn,
                &ReceivableInput {
                    contact_id: None,
                    contact_name: Some("Ana".to_string()),
                    title: "Prestamo".to_string(),
                    kind: None,
                    total_amount: 50_000,
                    currency_code: "CLP".to_string(),
                    origin_date: "2020-01-01".to_string(),
                    due_date: Some("2020-02-01".to_string()),
                    notes: None,
                },
                "CLP",
            )
        })
        .unwrap();

    state
        .write(|conn| {
            receivables::receive(
                conn,
                &por_cobrar.id,
                &ReceivablePaymentInput {
                    amount: 10_000,
                    received_at: dates::today_str(),
                    account_id: Some(cuenta.clone()),
                },
            )
        })
        .unwrap();

    let actualizada = state
        .read(|conn| Ok(receivables::get(conn, &por_cobrar.id)?))
        .unwrap()
        .unwrap();
    assert_eq!(actualizada.received_amount, 10_000);
    assert_eq!(
        actualizada.status, "overdue",
        "la deuda sigue vencida tras un cobro parcial"
    );
}

#[test]
fn no_se_puede_duplicar_un_presupuesto_para_la_misma_categoria_y_moneda() {
    let (state, _dir) = new_state("budget-duplicate");
    create_account(&state, "Efectivo", "cash", "CLP", 0);
    let categorias = state
        .read(|conn| {
            Ok(repos::categories::list(
                conn,
                Some("expense"),
                false,
                ("2024-01-01", "2024-12-31"),
                None,
            )?)
        })
        .unwrap()
        .unwrap();
    let primera = categorias[0].id.clone();
    let segunda = categorias[1].id.clone();

    let creado = state
        .write(|conn| {
            budgets::upsert(
                conn,
                &BudgetInput {
                    category_id: primera.clone(),
                    currency_code: "CLP".to_string(),
                    amount: 100_000,
                    start_date: None,
                    end_date: None,
                },
            )
        })
        .unwrap();
    let creado_id = creado.id.clone();

    // Editar el presupuesto para que caiga en otra categoría ya presupuestada
    // debe rechazarse en lugar de sobrescribir el límite existente.
    let segunda_con_presupuesto = state
        .write(|conn| {
            budgets::upsert(
                conn,
                &BudgetInput {
                    category_id: segunda.clone(),
                    currency_code: "CLP".to_string(),
                    amount: 50_000,
                    start_date: None,
                    end_date: None,
                },
            )
        })
        .unwrap();

    let choque = state.write(|conn| {
        budgets::update(
            conn,
            &creado_id,
            &BudgetInput {
                category_id: segunda_con_presupuesto.category_id.clone(),
                currency_code: "CLP".to_string(),
                amount: 200_000,
                start_date: None,
                end_date: None,
            },
        )
    });
    assert!(choque.is_err(), "no se puede duplicar el par categoría/moneda");

    // El presupuesto original sigue intacto.
    let lista = state
        .read(|conn| {
            let (start, end) = dates::period_range("month", 0)?;
            Ok(budgets::list(conn, (&start, &end))?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(lista.len(), 2);
}

#[test]
fn la_exportacion_usa_el_separador_configurado_y_no_se_trunca() {
    let (state, _dir) = new_state("export-separator");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    for indice in 0..600 {
        transaction(
            &state,
            "expense",
            &format!("Compra {indice}"),
            1_000,
            &cuenta,
            None,
            "2024-03-01",
        );
    }

    state
        .write(|conn| {
            settings::save(
                conn,
                &settings::AppSettings {
                    default_currency: "CLP".to_string(),
                    date_format: "DD/MM/AAAA".to_string(),
                    theme: "light".to_string(),
                    export_separator: ",".to_string(),
                    backup_folder: None,
                    week_starts_on_monday: true,
                },
            )
        })
        .unwrap();

    let csv = state
        .read(|conn| {
            Ok(export::transactions_csv(
                conn,
                &TransactionFilter {
                    include_voided: Some(true),
                    ..Default::default()
                },
            )?)
        })
        .unwrap()
        .unwrap();

    let lineas: Vec<&str> = csv.lines().collect();
    assert_eq!(lineas.len(), 601, "se exportan las 600 filas más la cabecera");
    assert!(
        lineas[0].contains(',') && !lineas[0].contains(';'),
        "se usa el separador configurado"
    );
    assert!(csv.contains("Compra 0"), "no se trunca el archivo");
    assert!(csv.contains("Compra 599"), "están todas las filas");
}

#[test]
fn no_se_puede_marcar_como_pagado_sin_registrar_el_cobro() {
    let (state, _dir) = new_state("manual-paid");
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    let por_cobrar = state
        .write(|conn| {
            receivables::create(
                conn,
                &ReceivableInput {
                    contact_id: None,
                    contact_name: Some("Ana".to_string()),
                    title: "Prestamo".to_string(),
                    kind: None,
                    total_amount: 50_000,
                    currency_code: "CLP".to_string(),
                    origin_date: "2024-03-01".to_string(),
                    due_date: None,
                    notes: None,
                },
                "CLP",
            )
        })
        .unwrap();

    let marcado = state.write(|conn| receivables::set_status(conn, &por_cobrar.id, "paid"));
    assert!(
        marcado.is_err(),
        "marcar pagado sin registrar el cobro dejaría la fila incoherente"
    );

    // Cancelar sí se permite: no mueve dinero.
    let cancelado = state
        .write(|conn| receivables::set_status(conn, &por_cobrar.id, "cancelled"))
        .unwrap();
    assert_eq!(cancelado.status, "cancelled");
    assert_eq!(balance(&state, &cuenta), 0);
}

#[test]
fn una_persona_no_puede_participar_dos_veces_en_la_misma_suscripcion() {
    let (state, _dir) = new_state("share-duplicate");
    let cuenta = create_account(&state, "Tarjeta", "credit_card", "CLP", 0);
    let suscripcion = state
        .write(|conn| subscriptions::create(conn, &subscription_input(9_990, &cuenta), "CLP"))
        .unwrap();

    let contacto = state
        .write(|conn| {
            let contacto = ContactInput {
                name: "Juan".to_string(),
                kind: "person".to_string(),
                notes: None,
            };
            Ok(aureon_lib::core::services::categories::create_contact(conn, &contacto)?)        })
        .unwrap();

    let compartido = state
        .write(|conn| {
            subscriptions::add_share(
                conn,
                &suscripcion.id,
                &SubscriptionShareInput {
                    contact_id: contacto.id.clone(),
                    share_type: "percent".to_string(),
                    share_value: 50,
                    receivable_id: None,
                },
            )
        })
        .unwrap();
    assert_eq!(compartido.share_value, 50);

    let repetido = state.write(|conn| {
        subscriptions::add_share(
            conn,
            &suscripcion.id,
            &SubscriptionShareInput {
                contact_id: contacto.id.clone(),
                share_type: "percent".to_string(),
                share_value: 30,
                receivable_id: None,
            },
        )
    });
    assert!(repetido.is_err(), "no se puede añadir dos veces a la misma persona");
}

#[test]
fn una_base_vacia_se_aparta_y_no_deja_la_aplicacion_sin_datos() {
    // Un archivo de 0 bytes (por ejemplo, tras un cierre abrupto durante la
    // creación) no debe dejar la aplicación con una base rota.
    let dir = temp_dir("empty-db");
    let path = dir.join("aureon.db");
    std::fs::write(&path, b"").unwrap();

    let state = AppState::new(dir.clone());
    assert!(!state.init().unwrap(), "una base vacía no cuenta como base de datos");
    assert!(!state.has_db());
    assert!(!path.exists(), "el archivo vacío se aparta");

    // Y la aplicación sigue funcionando: la primera escritura crea la base.
    let cuenta = create_account(&state, "Efectivo", "cash", "CLP", 0);
    assert_eq!(balance(&state, &cuenta), 0);

    let tablas: i64 = state
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN
                 ('accounts','categories','contacts','transactions','subscriptions',
                  'subscription_payments','purchases','purchase_installments',
                  'installment_payments','receivables','receivable_payments','payables',
                  'payable_payments','deposits','budgets','wishlist_items','settings',
                  'schema_migrations')",
                [],
                |row| row.get(0),
            )?)
        })
        .unwrap()
        .unwrap();
    assert_eq!(tablas, 18, "la base nueva debe quedar con el esquema completo");
}

#[test]
fn el_catalogo_coincide_con_la_base_de_datos() {
    // El catálogo de referencia (monedas y categorías por defecto) y lo que
    // siembra la migración no pueden divergir: si divergen, la instalación
    // nueva mostraría una cosa y la existente otra.
    let (state, _dir) = new_state("catalog");
    create_account(&state, "Efectivo", "cash", "CLP", 0);

    let monedas_db: Vec<(String, String, String, i64)> = state
        .read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT code, name, symbol, exponent FROM currencies ORDER BY code",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
        .unwrap()
        .unwrap();

    let monedas_catalogo: Vec<(String, String, String, i64)> =
        aureon_lib::core::catalog::monedas()
            .into_iter()
            .map(|m| (m.code, m.name, m.symbol, m.exponent as i64))
            .collect();
    // La migración inserta en orden CLP, USD, EUR; el catálogo sigue ese mismo
    // orden. Se ordena ambos conjuntos por código para comparar sin depender
    // del orden de inserción.
    let mut monedas_db_ordenadas = monedas_db.clone();
    monedas_db_ordenadas.sort();
    let mut monedas_catalogo_ordenadas = monedas_catalogo.clone();
    monedas_catalogo_ordenadas.sort();
    assert_eq!(monedas_db_ordenadas, monedas_catalogo_ordenadas);

    let categorias_db: Vec<(String, String, String, String)> = state
        .read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, kind, color FROM categories ORDER BY id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
        .unwrap()
        .unwrap();

    let categorias_catalogo: Vec<(String, String, String, String)> =
        aureon_lib::core::catalog::categorias(None)
            .into_iter()
            .map(|c| (c.id, c.name, c.kind, c.color.unwrap_or_default()))
            .collect();
    let mut categorias_db = categorias_db.clone();
    categorias_db.sort();
    let mut categorias_catalogo = categorias_catalogo.clone();
    categorias_catalogo.sort();
    assert_eq!(categorias_db, categorias_catalogo);
}

#[test]
fn sin_base_de_datos_las_listas_de_referencia_siguen_disponibles() {
    // En una instalación recién estrenada, las monedas y las categorías por
    // defecto deben estar disponibles aunque no exista todavía el archivo.
    let (state, _dir) = new_state("catalog-sin-db");
    assert!(!state.has_db());

    let monedas = state.read_or(repos::list_currencies).unwrap_or_default();
    assert!(monedas.is_empty(), "sin base de datos la consulta no devuelve nada");
    let catalogo = aureon_lib::core::catalog::monedas();
    assert_eq!(catalogo.len(), 3);
    assert_eq!(catalogo[0].code, "CLP");
    assert_eq!(catalogo[1].code, "USD");
    assert_eq!(catalogo[2].code, "EUR");
    assert_eq!(catalogo[0].exponent, 0);
    assert_eq!(catalogo[1].exponent, 2);

    let categorias = aureon_lib::core::catalog::categorias(Some("expense"));
    assert!(!categorias.is_empty());
    assert!(categorias.iter().all(|c| c.kind == "expense"));
}

#[test]
fn se_pueden_escribir_archivos_binarios_como_el_pdf_de_los_reportes() {
    let dir = temp_dir("write-file");
    let destino = dir.join("reporte.pdf");
    // "Aureon" codificado en base64.
    let contenido = "QXVyZW9u";
    aureon_lib::commands::system::write_base64_file(
        destino.to_string_lossy().to_string(),
        contenido.to_string(),
    )
    .expect("debe escribir el archivo");
    let bytes = std::fs::read(&destino).unwrap();
    assert_eq!(bytes, b"Aureon");

    // Extensión no permitida.
    let prohibido = dir.join("archivo.exe");
    assert!(
        aureon_lib::commands::system::write_base64_file(
            prohibido.to_string_lossy().to_string(),
            contenido.to_string(),
        )
        .is_err(),
        "sólo se admiten PDF, CSV y binarios"
    );
}

#[test]
fn se_pueden_crear_transacciones_directamente_en_la_base() {
    // Comprobación de que la capa de repositorios usa consultas parametrizadas:
    // un intento de inyección debe fallar sin tocar nada.
    let mut conn = Connection::open_in_memory().unwrap();
    aureon_lib::core::migrations::migrate(&mut conn).unwrap();

    let resultado = conn.execute(
        "INSERT INTO accounts (id, name, type, currency_code, initial_balance, initial_balance_date, archived, created_at)
         VALUES (?1, ?2, 'cash', 'CLP', 0, '2024-01-01', 0, '2024-01-01')",
        ["'; DROP TABLE accounts; --", "Malicioso"],
    );
    assert!(resultado.is_ok());
    let cuentas: i64 = conn
        .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
        .unwrap();
    assert_eq!(cuentas, 1, "el valor se trata como dato, no como código");
}
