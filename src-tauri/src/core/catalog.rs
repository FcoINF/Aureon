//! Catálogos de referencia de Aureon.
//!
//! Las monedas y las categorías por defecto son datos de referencia, no datos
//! del usuario: deben estar disponibles aunque todavía no exista la base de
//! datos (por ejemplo, en la pantalla de Configuración de una instalación
//! recién estrenada).
//!
//! La migración inicial siembra exactamente estos mismos valores; la prueba
//! `el_catalogo_coincide_con_la_base_de_datos` verifica que no diverjan.

use crate::core::models::{Category, Currency};

pub struct CatalogCurrency {
    pub code: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
    pub exponent: u8,
}

pub struct CatalogCategory {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: &'static str,
    pub color: &'static str,
}

/// Monedas admitidas por Aureon.
pub const MONEDAS: [CatalogCurrency; 3] = [
    CatalogCurrency {
        code: "CLP",
        name: "Peso chileno",
        symbol: "$",
        exponent: 0,
    },
    CatalogCurrency {
        code: "USD",
        name: "Dólar estadounidense",
        symbol: "US$",
        exponent: 2,
    },
    CatalogCurrency {
        code: "EUR",
        name: "Euro",
        symbol: "€",
        exponent: 2,
    },
];

/// Categorías que Aureon propone al empezar.
pub const CATEGORIAS: [CatalogCategory; 15] = [
    CatalogCategory { id: "cat-alimentacion", name: "Alimentación", kind: "expense", color: "#C2703D" },
    CatalogCategory { id: "cat-transporte", name: "Transporte", kind: "expense", color: "#4C7BA8" },
    CatalogCategory { id: "cat-vivienda", name: "Vivienda", kind: "expense", color: "#6C6F86" },
    CatalogCategory { id: "cat-servicios", name: "Servicios básicos", kind: "expense", color: "#3F8A7A" },
    CatalogCategory { id: "cat-salud", name: "Salud", kind: "expense", color: "#C05A6B" },
    CatalogCategory { id: "cat-educacion", name: "Educación", kind: "expense", color: "#7A62A8" },
    CatalogCategory { id: "cat-entretenimiento", name: "Entretenimiento", kind: "expense", color: "#C08A3E" },
    CatalogCategory { id: "cat-compras", name: "Compras online", kind: "expense", color: "#B0674F" },
    CatalogCategory { id: "cat-suscripciones", name: "Suscripciones", kind: "expense", color: "#8A6F9C" },
    CatalogCategory { id: "cat-tecnologia", name: "Tecnología", kind: "expense", color: "#4C7BA8" },
    CatalogCategory { id: "cat-ropa", name: "Ropa", kind: "expense", color: "#B0768C" },
    CatalogCategory { id: "cat-ahorro", name: "Ahorro", kind: "expense", color: "#3F8A7A" },
    CatalogCategory { id: "cat-salud-otros", name: "Otros gastos", kind: "expense", color: "#7B7F8A" },
    CatalogCategory { id: "cat-laboral", name: "Ingresos laborales", kind: "income", color: "#2F7A5A" },
    CatalogCategory { id: "cat-ingresos-otros", name: "Otros ingresos", kind: "income", color: "#3F8A7A" },
];

/// Monedas del catálogo, en formato de modelo.
pub fn monedas() -> Vec<Currency> {
    MONEDAS
        .iter()
        .map(|item| Currency {
            code: item.code.to_string(),
            name: item.name.to_string(),
            symbol: item.symbol.to_string(),
            exponent: item.exponent,
        })
        .collect()
}

/// Categorías del catálogo, opcionalmente filtradas por tipo.
/// Las del catálogo nunca están archivadas.
pub fn categorias(kind: Option<&str>) -> Vec<Category> {
    CATEGORIAS
        .iter()
        .filter(|item| kind.map(|k| k == item.kind).unwrap_or(true))
        .map(|item| Category {
            id: item.id.to_string(),
            name: item.name.to_string(),
            kind: item.kind.to_string(),
            color: Some(item.color.to_string()),
            archived: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            spent: Some(0),
            budget: None,
        })
        .collect()
}
