extern crate arguably;
extern crate edit;
extern crate trash;

use arguably::ArgParser;
use std::path::Path;
use std::process::exit;
use std::env;


const HELP: &str = "
Usage: vimv [FLAGS] [OPTIONS] [ARGUMENTS]

  This utility lets you batch rename files using a text editor. Files to be
  renamed should be supplied as a list of command-line arguments, e.g.

    $ vimv *.mp3

  The list of files will be opened in the editor specified by the $EDITOR
  environment variable, one filename per line. Edit the list, save, and exit.
  The files will be renamed to the edited filenames. Directories along the
  renamed path will be created as required.

  Use the --force/-f flag to overwrite existing files. Existing directories
  will never be overwritten. (If you attempt to overwrite a directory the
  program will exit with an error message and a non-zero status code.)

  You can delete files by 'renaming' them to a blank line, but only if the
  --delete/-d flag has been specified. Deleted files are moved to the system's
  trash/recycle bin.

Arguments:
  [files]                   List of files to rename.

Options:
  -e, --editor <name>       Specify the editor to use.

Flags:
  -d, --delete              Enable file deletion.
  -f, --force               Force overwrite existing files.
  -h, --help                Print this help text.
  -v, --version             Print the version number.
";


fn main() {
    let mut parser = ArgParser::new()
        .helptext(HELP)
        .version(env!("CARGO_PKG_VERSION"))
        .flag("force f")
        .flag("delete d")
        .option("editor e");

    // Parse the command line arguments.
    if let Err(err) = parser.parse() {
        err.exit();
    }
    if parser.args.len() == 0 {
        exit(0);
    }

    // Use the --editor option if present to set $VISUAL.
    if let Some(editor) = parser.value("editor") {
        env::set_var("VISUAL", editor);
    }

    // Assemble the input filenames and verify they all really exist.
    let input_filenames: Vec<&str> = parser.args.iter().map(|s| s.trim()).collect();
    for input_filename in &input_filenames {
        if !Path::new(input_filename).exists() {
            eprintln!("Error: the input file '{}' does not exist", input_filename);
            exit(1);
        }
    }

    // Fetch the string of output filenames from the editor.
    let input_string = parser.args.join("\n");
    let output_string = match edit::edit(input_string) {
        Ok(edited) => edited,
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
        }
    };

    // Sanity check - verify that we have equal numbers of input and output filenames.
    let output_filenames: Vec<&str> = output_string.lines().map(|s| s.trim()).collect();
    if output_filenames.len() != input_filenames.len() {
        eprintln!(
            "Error: number of input filenames ({}) does not match number of output filenames ({})",
            input_filenames.len(),
            output_filenames.len()
        );
        exit(1);
    }

    // Permissions check - verify that we have permission to perform all operations.
    // - We never overwrite directories.
    // - We only overwrite files if the --force flag has been set.
    // - We only delete files if the --delete flag has been set.
    for (input_filename, output_filename) in input_filenames.iter().zip(output_filenames.iter()) {
        let output_path = Path::new(output_filename);
        if input_filename == output_filename {
            continue;
        } else if output_path.is_dir() {
            eprintln!("Error: cannot overwrite directory '{}'", output_filename);
            exit(1);
        } else if output_filename.is_empty() {
            if !parser.found("delete") {
                eprintln!("Error: use the --delete flag to enable file deletion");
                exit(1);
            }
        } else if output_path.is_file() {
            if !parser.found("force") {
                eprintln!(
                    "Error: the output file '{}' already exists, use --force to overwrite",
                    output_filename
                );
                exit(1);
            }
        }
    }

    // Operations loop. We haven't made any changes to the file system up to this point.
    for (input_filename, output_filename) in input_filenames.iter().zip(output_filenames.iter()) {
        if input_filename == output_filename {
            continue;
        } else if output_filename.is_empty() {
            delete_file(input_filename);
        } else {
            move_file(input_filename, output_filename);
        }
    }
}


// Move the specified file to the system trash/recycle bin.
fn delete_file(input_filename: &str) {
    if let Err(err) = trash::delete(input_filename) {
        eprintln!("Error: {}", err);
        exit(1);
    }
}


// Rename `input_filename` to `output_filename`.
fn move_file(input_filename: &str, output_filename: &str) {
    if let Some(parent_path) = Path::new(output_filename).parent() {
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
