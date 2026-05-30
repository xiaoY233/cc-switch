fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let response = cc_switch_lib::cli::run(&args);
    println!(
        "{}",
        serde_json::to_string(&response).expect("serialize CLI response")
    );
}
