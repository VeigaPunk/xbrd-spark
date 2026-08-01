fn main() {
    if let Err(e) = xbrd_spark::run_cli() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
