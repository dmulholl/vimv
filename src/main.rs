use arguably::ArgParser;
use std::path::Path;
use std::process::exit;
use std::env;
use std::collections::HashSet;
use rand::Rng;


const HELP: &str = "
Usage: vimv [files]

  This utility lets you batch rename files using a text editor. Files to be
  renamed should be supplied as a list of command-line arguments, e.g.

    $ vimv *.mp3

  The list of files will be opened in the editor specified by the $EDITOR
  environment variable, one filename per line. Edit the list, save, and exit.
  The files will be renamed to the edited filenames. Directories along the
  renamed path will be created as required.

  Vimv supports cycle-renaming. You can safely rename A to B, B to C, and C
  to A in a single operation.

  Use the --force flag to overwrite existing files that aren't part of a
  renaming cycle. (Existing directories are never overwritten. If you attempt
  to overwrite a directory the program will exit with an error message and a
  non-zero status code.)

  You can delete a file by 'renaming' it to a blank line, but only if the
  --delete flag has been specified. Deleted files are moved to the system's
  trash/recycle bin.

Arguments:
  [files]                   List of files to rename.

Options:
  -e, --editor <name>       Specify the editor to use.

Flags:
  -d, --delete              Enable file deletion.
  -f, --force               Overwrite existing files.
  -h, --help                Print this help text.
  -v, --version             Print the version number.
";


fn main() {
    let mut parser = ArgParser::new()
        .helptext(HELP)
        .version(env!("CARGO_PKG_VERSION"))
        .flag("force f")
        .flag("delete d")
        .option("editor e", "");

    // Parse the command line arguments.
    if let Err(err) = parser.parse() {
        err.exit();
    }
    if parser.args.len() == 0 {
        exit(0);
    }

    // Use the --editor option if present to set $VISUAL.
    if parser.found("editor") {
        env::set_var("VISUAL", parser.value("editor"));
    }

    // Assemble the list of input filenames and verify that they all exist.
    let input_files: Vec<String> = parser.args.iter().map(|s| s.trim().to_string()).collect();
    for input_file in &input_files {
        if !Path::new(input_file).exists() {
            eprintln!("Error: the input file '{}' does not exist.", input_file);
            exit(1);
        }
    }

    // Fetch the string of output filenames from the editor.
    let editor_input = parser.args.join("\n");
    let editor_output = match edit::edit(editor_input) {
        Ok(edited) => edited,
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
        }
    };

    // Sanity check - verify that we have equal numbers of input and output filenames.
    let output_files: Vec<String> = editor_output.lines().map(|s| s.trim().to_string()).collect();
    if output_files.len() != input_files.len() {
        eprintln!(
            "Error: the number of input filenames ({}) does not match the number of output filenames ({}).",
            input_files.len(),
            output_files.len()
        );
        exit(1);
    }

    // Sanity check - verify that the (non-empty) output filenames are unique.
    let mut set = HashSet::new();
    for output_file in output_files.iter().filter(|s| !s.is_empty()) {
        if set.contains(output_file) {
            eprintln!("Error: the filename '{}' appears in the output list multiple times.", output_file);
            exit(1);
        } else {
            set.insert(output_file);
        }
    }

    // List of files to delete.
    let mut delete_list: Vec<&str> = Vec::new();

    // List of rename operations as (src, dst) tuples.
    let mut rename_list: Vec<(String, String)> = Vec::new();

    // Populate the todo lists.
    for (input_file, output_file) in input_files.iter().zip(output_files.iter()) {
        if input_file == output_file {
            continue;
        } else if Path::new(output_file).is_dir() {
            eprintln!("Error: cannot overwrite the existing directory '{}'.", output_file);
            exit(1);
        } else if output_file.is_empty() {
            if parser.found("delete") {
                delete_list.push(input_file);
            } else {
                eprintln!("Error: use the --delete flag to enable file deletion.");
                exit(1);
            }
        } else if Path::new(output_file).is_file() {
            if input_files.contains(output_file) {
                rename_list.push((input_file.to_string(), output_file.to_string()));
            } else if parser.found("force") {
                rename_list.push((input_file.to_string(), output_file.to_string()));
            } else {
                eprintln!(
                    "Error: the output file '{}' already exists, use --force to overwrite it.",
                    output_file
                );
                exit(1);
            }
        } else {
            rename_list.push((input_file.to_string(), output_file.to_string()));
        }
    }

    // Check for cycles. If we find src being renamed to dst where dst is one of the input
    // filenames, we rename src to tmp instead and later rename tmp to dst.
    for i in 0..rename_list.len() {
        if input_files.contains(&rename_list[i].1) {
            let temp_file = get_temp_filename(&rename_list[i].0);
            rename_list.push((temp_file.clone(), rename_list[i].1.clone()));
            rename_list[i].1 = temp_file
        }
    }

    // Deletion loop. We haven't make any changes to the file system up to this point.
    for input_file in delete_list {
        delete_file(input_file);
    }

    // Rename loop.
    for (input_file, output_file) in rename_list {
        move_file(&input_file, &output_file);
    }
}


// Generate a unique temporary filename.
fn get_temp_filename(base: &str) -> String {
    let mut rng = rand::thread_rng();
    for _ in 0..10 {
        let candidate = format!("{}.vimv_temp_{:04}", base, rng.gen_range(0, 10_000));
        if !Path::new(&candidate).exists() {
            return candidate;
        }
    }
    eprintln!("Error: cannot generate a temporary filename of the form '{}.vimv_temp_XXXX'.", base);
    exit(1);
}


// Move the specified file to the system trash/recycle bin.
fn delete_file(input_file: &str) {
    if let Err(err) = trash::delete(input_file) {
        eprintln!("Error: {}", err);
        exit(1);
    }
}


// Rename `input_file` to `output_file`.
fn move_file(input_file: &str, output_file: &str) {
    if let Some(parent_path) = Path::new(output_file).parent() {
        if !parent_path.is_dir() {
            if let Err(err) = std::fs::create_dir_all(parent_path) {
                eprintln!("Error: {}", err);
                exit(1);
            }
        }
    }
    if let Err(err) = std::fs::rename(input_file, output_file) {
        eprintln!("Error: {}", err);
        exit(1);
    }
}
