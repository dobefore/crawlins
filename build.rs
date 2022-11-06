use std::env;
fn main() {
    let cy = "chengyu";
    let key = format!("CARGO_FEATURE_{}", cy).to_uppercase();
    if env::var_os(key).is_some() {
        println!("cargo:rustc-cfg=feature=\"{}\"", cy)
    }
}
