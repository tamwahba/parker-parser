use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

fn main() {
    use std::io::Write;

    let args: Vec<_> = env::args().collect();
    if args.len() < 3 {
        println!(
            "error: missing arguments.\nusage: {} <input_file> <output_file>",
            args[0]
        );
        return;
    }

    let inp = match File::open(&args[1]) {
        Ok(file) => file,
        Err(e) => panic!("couldn't open file: {}", e.description()),
    };

    let mut out = match File::create(&args[2]) {
        Ok(file) => file,
        Err(e) => panic!("couldn't create file: {}", e.description()),
    };

    let reader = BufReader::new(inp);

    let mut missed = 0;
    let mut total = 0;
    for (idx, line) in reader.lines().flat_map(|res| res.ok()).enumerate() {
        let rules = parkerparser::parse_str(&line);
        total = idx;

        if rules.is_empty() {
            missed += 1;
        }

        write!(out, "line {}: {}\n{:?}\n\n", idx + 1, line, rules)
            .unwrap_or_else(|e| panic!("ln {:?}", e));
    }

    println!(
        "total: {}, missed: {}, miss rate: {:.2}%",
        total,
        missed,
        (missed as f32 / total as f32) * 100.0
    );
}
