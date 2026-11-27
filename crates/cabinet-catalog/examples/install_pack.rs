//! Installs a material pack: `cargo run -p cabinet-catalog --example install_pack -- PACK.zip APP_DIR`.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    match cabinet_catalog::packs::install_pack(a[1].as_ref(), a[2].as_ref()) {
        Ok(p) => println!("installed {} {} ({} pictures)", p.id, p.version, p.images),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
    }
}
