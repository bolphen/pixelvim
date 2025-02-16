#[cfg(not(target_arch = "wasm32"))]
const HELP: &str = r#"USAGE
    pixelvim [OPTIONS] [<path> ...]

OPTIONS
    -h, --help           Prints help
    -v, --version        Prints version
    -u <config>          Use config file
"#;

#[cfg(not(target_arch = "wasm32"))]
fn handle_arguments() -> Result<(), String> {
    use std::path::PathBuf;

    let mut args = pico_args::Arguments::from_env();
    if args.contains(["-h", "--help"]) {
        println!("{}", HELP);
    } else if args.contains(["-v", "--version"]) {
        println!("pixelvim v{}", pixelvim::VERSION);
    } else {
        let config = args
            .opt_value_from_str::<_, PathBuf>("-u")
            .map_err(|e| e.to_string())?;
        pixelvim::init(
            config,
            args.finish().into_iter().map(|p| p.into()).collect(),
        );
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(e) = handle_arguments() {
        eprintln!("pixelvim: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    pixelvim::init(None, Vec::new());
}
