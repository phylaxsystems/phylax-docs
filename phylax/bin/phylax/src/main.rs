fn main() {
    if let Err(err) = phylax::cli::run() {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
