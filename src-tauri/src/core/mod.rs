//! Núcleo de Aureon: base de datos, modelos, reglas financieras y servicios.
//!
//! Esta capa no depende de Tauri: puede probarse de forma aislada.

pub mod dates;
pub mod db;
pub mod error;
pub mod migrations;
pub mod models;
pub mod money;
pub mod repos;
pub mod services;
