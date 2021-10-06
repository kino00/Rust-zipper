extern crate zipper;

use std::env;

use zipper::encode;

/*
 コマンドライン引数で入力を受け付けている。
 */
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        let usage = r#"
        compress input -> output
    "#;

        println!("{}", usage);
        panic!("No file names");
    }
    let input_file = &args[1];
    let output_file = &args[2];

    encode(&input_file, &output_file)
        .unwrap_or_else(|err| eprintln!("IO Error => {}", err));
}
