fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match librarian_ingest::cli::run(&args) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
