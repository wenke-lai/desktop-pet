// Run the same read-only query used by the pet, printing every returned field.
#[allow(dead_code)]
#[path = "../src/codex.rs"]
mod codex;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    match codex::fetch().await {
        Ok(snapshot) => println!("{}", serde_json::to_string_pretty(&snapshot).unwrap()),
        Err(error) => { eprintln!("{error}"); std::process::exit(1); }
    }
}
