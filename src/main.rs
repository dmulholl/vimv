extern crate arguably;
use arguably::ArgParser;


fn main() {

    let mut parser = ArgParser::new()
        .helptext("Usage: vimv...")
        .version("0.1");

    if let Err(err) = parser.parse() {
        err.exit();
    }

    for arg in parser.args {
        println!("{}", arg);
    }
}
