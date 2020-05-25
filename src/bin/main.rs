use std::io::Write;
use httpclient::request;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        writeln!(std::io::stderr(), "Usage: worker URL").unwrap();
        writeln!(
            std::io::stderr(),
            "Example: {} https://en.wikipedia.org/wiki/Rust_(programming_language)",
            args[0]
        )
        .unwrap();
        std::process::exit(1);
    }
    let response = request(&args[1])
        .unwrap();
    println!("{:?}", response);
}
