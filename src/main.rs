#[cfg(not(target_arch = "wasm32"))]
const HELP: &str = "\
USAGE
    pixelvim [OPTIONS] [<path> ...]

OPTIONS
    -h, --help               Prints help
    -v, --version            Prints version
    -u <config>              Use config file

    -n <name>                Create new file

    --prefer-x11             Prefer X11/XWayland over native Wayland
    --transparent-window     Allow transparent window; platform-dependent
";

#[cfg(not(target_arch = "wasm32"))]
fn concat_wd<P>(base: &Option<std::path::PathBuf>, path: P) -> std::path::PathBuf
where
    P: AsRef<std::path::Path> + Into<std::path::PathBuf>,
{
    base.as_ref()
        .map(|b| {
            let mut b = b.clone();
            b.push(path.as_ref());
            b
        })
        .unwrap_or(path.into())
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_arguments() -> Result<(), String> {
    use std::path::PathBuf;

    let mut args = pico_args::Arguments::from_env();
    if args.contains(["-h", "--help"]) {
        print!("{}", HELP);
    } else if args.contains(["-v", "--version"]) {
        println!("pixelvim v{}", pixelvim::VERSION);
    } else {
        let launch_path = std::env::current_dir().ok();
        let prefer_x11 = args.contains("--prefer-x11");
        let transparent_window = args.contains("--transparent-window");
        let config_file = args
            .opt_value_from_str::<_, PathBuf>("-u")
            .map_err(|e| e.to_string())?;
        let new_file = args
            .opt_value_from_str::<_, PathBuf>("-n")
            .map_err(|e| e.to_string())?
            .map(|p| concat_wd(&launch_path, p));
        let paths = args
            .finish()
            .into_iter()
            .map(|p| concat_wd(&launch_path, p))
            .collect();
        let config = pixelvim::Config {
            prefer_x11,
            transparent_window,
            config_file,
            new_file,
        };
        pixelvim::init(config, paths);
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
    pixelvim::init(Default::default(), Vec::new());
}
