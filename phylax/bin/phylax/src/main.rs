#![allow(missing_docs)]

fn main() {
    use phylax::cli::Cli;

    phylax::sigsegv_handler::install();

    // Enable backtraces unless a RUST_BACKTRACE value has already been explicitly provided.
    if std::env::var_os("RUST_BACKTRACE").is_none() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    if let Err(err) = Cli::parse_args().run(|builder, _| async {
        let handle = builder.launch().await?;
        if let Some(exit_fut) = handle.exit_future {
            exit_fut.await.map_err(|err| err.into())
        } else {
            eprintln!("Error, no exit future found");
            std::process::exit(1);
        }
    }) {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
