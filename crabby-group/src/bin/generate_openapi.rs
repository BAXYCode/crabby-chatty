use std::{env, fs, path::PathBuf};

use utoipa::openapi::{self, Contact};

fn main() {
    let (_, mut openapi) = crabby_group::api::router().split_for_parts();
    //Change contact info to my own
    let contact = Contact::builder()
        .email(Some("lainebenjamin3@gmail.com"))
        .name(Some("Benjamin Laine"))
        .build();
    openapi.info.contact = Some(contact);
    let json = openapi
        .to_pretty_json()
        .expect("failed to serialize OpenAPI spec");

    let out = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("openapi.json"));

    fs::write(&out, json)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", out.display()));

    eprintln!("wrote OpenAPI spec to {}", out.display());
}
