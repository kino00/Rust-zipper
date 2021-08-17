extern crate zipper;

use std::env;

use zipper::encode;

fn main() -> Result<(), std::io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        let usage = r#"
        compress input -> output
    "#;

        println!("{}", usage);
        return Ok(());
    }
    let input_file = &args[1];
    let output_file = &args[2];

    encode(&input_file, &output_file)?;
    Ok(())
}
