use std::{fs, path::Path};

use crabby_specs::WsApi;

fn main() -> std::io::Result<()> {
    let spec = WsApi::asyncapi_spec();

    //stringify
    let json = serde_json::to_string_pretty(&spec).expect("Build json spec");

    let output_dir = Path::new("./asyncapi");
    fs::create_dir_all(output_dir)?;

    let output_path = output_dir.join("ws_api.json");
    fs::write(&output_path, &json)?;
    println!("✅ Generated: {}", output_path.display());
    println!("\n📄 Specification preview:");
    println!("{}", json);
    Ok(())
}
