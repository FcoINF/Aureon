// Evita que se abra una consola adicional en Windows en modo publicación.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    aureon_lib::run()
}
