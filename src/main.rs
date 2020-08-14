extern crate arguably;
extern crate edit;

use arguably::ArgParser;
use std::path::Path;
use std::process::exit;


const HELP: &str = "
Usage: vimv [FLAGS] [ARGUMENTS]

  This utility lets you batch rename files using a text editor. Files to be
  renamed should be supplied as a list of command-line arguments, e.g.

    $ vimv *.mp3

  The list of files will be opened in the editor specified by the $EDITOR
  environment variable, one filename per line. Edit the list, save, and exit.
  The files will be renamed to the edited filenames. Directories along the
  path will be created as required.

  Use the --force flag to overwrite existing files. Existing directories will
  not be overwritten. (If you attempt to overwrite a directory the program
  will exit with an error message and a non-zero status code.)

Arguments:
  [files]               List of files to rename.

Flags:
  -f, --force           Force overwrite existing files.
  -h, --help            Print this help text.
  -v, --version         Print the version number.
";


fn main() {
    let mut parser = ArgParser::new()
        .helptext(HELP)
        .version(env!("CARGO_PKG_VERSION"))
        .flag("force f");

    if let Err(err) = parser.parse() {
        err.exit();
    }

    if parser.args.len() == 0 {
        exit(0);
    }

    let input_filenames: Vec<&str> = parser.args.iter().map(|s| s.trim()).collect();
    for input_filename in &input_filenames {
        if !Path::new(input_filename).exists() {
            eprintln!("Error: the input file '{}' does not exist", input_filename);
            exit(1);
        }
    }

    let input_string = parser.args.join("\n");
    let output_string = match edit::edit(input_string) {
        Ok(edited) => edited,
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
        }
    };

    let output_filenames: Vec<&str> = output_string.trim().lines().map(|s| s.trim()).collect();
    if output_filenames.len() != input_filenames.len() {
        eprintln!("Error: number of input filenames does not match number of output filenames");
        exit(1);
    }

    for (input_filename, output_filename) in input_filenames.iter().zip(output_filenames.iter()) {
        move_file(input_filename, output_filename, parser.found("force").unwrap());
    }
}


fn move_file(input_filename: &str, output_filename: &str, overwrite: bool) {
    if input_filename == output_filename {
        return;
    }

    let output_path = Path::new(output_filename);
    if output_path.is_dir() {
        eprintln!("Error: cannot overwrite directory '{}'", output_filename);
        exit(1);
    }
    if output_path.is_file() && !overwrite {
        eprintln!(
            "Error: the output file '{}' already exists, use --force to overwrite",
            output_filename
        );
        exit(1);
    }

    if let Some(parent_path) = output_path.parent() {
        if !parent_path.is_dir() {
            if let Err(err) = std::fs::create_dir_all(parent_path) {
                eprintln!("Error: {}", err);
                exit(1);
            }
        }
    }

    if let Err(err) = std::fs::rename(input_filename, output_filename) {
        eprintln!("Error: {}", err);
        exit(1);
    }
}
