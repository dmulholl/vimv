use arguably::ArgParser;
use std::path::Path;
use std::process::exit;
use std::env;
use std::collections::HashSet;
use rand::Rng;
use std::io::Read;
use colored::*;


const HELPTEXT: &str = "
Usage: vimv [files]

  This utility lets you batch-rename files using a text editor. Files to be
  renamed should be supplied as a list of command-line arguments, e.g.

    $ vimv *.mp3

  The list of files will be opened in the editor specified by the $EDITOR
  environment variable, one filename per line. Edit the list, save, and exit.
  The files will be renamed to the edited filenames. Directories along the
  new paths will be created as required.

  Vimv supports cycle-renaming. You can safely rename A to B, B to C, and C
  to A in a single operation.

  Use the --force flag to overwrite existing files that aren't part of a
  renaming cycle. (Existing directories are never overwritten. If you attempt
  to overwrite a directory the program will exit with an error message and a
  non-zero status code.)

  You can delete a file or directory by prefixing its name with a `#` symbol.
  Deleted files are moved to the system's trash/recycle bin.

Arguments:
  [files]                   List of files to rename.

Options:
  -e, --editor <name>       Specify the editor to use. Overrides $EDITOR.

Flags:
  -f, --force               Overwrite existing files.
  -h, --help                Print this help text and exit.
  -q, --quiet               Only report errors.
  -s, --stdin               Read the list of input files from standard input.
  -v, --version             Print the version number and exit.
";


fn main() {
    let mut parser = ArgParser::new()
        .helptext(HELPTEXT)
        .version(env!("CARGO_PKG_VERSION"))
        .flag("force f")
        .flag("quiet q")
        .flag("stdin s")
        .option("editor e", "");

    // Parse the command line arguments.
    if let Err(err) = parser.parse() {
        err.exit();
    }

    // Use the --editor option if present to set $VISUAL.
    if parser.found("editor") {
        env::set_var("VISUAL", parser.value("editor"));
    }

    // Assemble the list of input filenames.
    let mut input_files: Vec<String> = parser.args.clone();

    // If the --stdin flag has been set, try reading from standard input.
    if parser.found("stdin") {
        let mut buffer = String::new();
        if let Err(err) = std::io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error: failed to read filenames from standard input: {}", err);
            exit(1);
        }
        if !buffer.trim().is_empty() {
            input_files.extend(buffer.lines().map(|s| s.to_string()));
        }
    }

    // Bail if we have no input filenames to process.
    if input_files.is_empty() {
        exit(0);
    }

    // Sanity check - verify that no input filename begins with '#'.
    for input_file in &input_files {
        if input_file.starts_with("#") {
            eprintln!("Error: input filenames cannot begin with '#'.");
            exit(1);
        }
    }

    // Sanity check - verify that all the input files exist.
    for input_file in &input_files {
        if !Path::new(input_file).exists() {
            eprintln!("Error: the input file '{}' does not exist.", input_file);
            exit(1);
        }
    }

    // Sanity check - verify that the input filenames are unique.
    let mut input_set = HashSet::new();
    for input_file in &input_files {
        if input_set.contains(input_file) {
            eprintln!("Error: the filename '{}' appears in the input list multiple times.", input_file);
            exit(1);
        }
        input_set.insert(input_file);
    }

    // Fetch the output filenames from the editor.
    let editor_input = input_files.join("\n") + "\n";
    let editor_output = match edit::edit(editor_input) {
        Ok(edited) => edited.trim().to_string(),
        Err(err) => {
            eprintln!("Error: {}", err);
            exit(1);
        }
    };
    let output_files: Vec<String> = editor_output.lines().map(|s| s.to_string()).collect();

    // Sanity check - verify that we have equal numbers of input and output filenames.
    if output_files.len() != input_files.len() {
        eprintln!(
            "Error: the number of input filenames ({}) does not match the number of output filenames ({}).",
            input_files.len(),
            output_files.len()
        );
        exit(1);
    }

    // Sanity check - verify that the output filenames are unique.
    let mut case_sensitive_output_set = HashSet::new();
    for output_file in output_files.iter().filter(|s| !s.starts_with("#")) {
        if case_sensitive_output_set.contains(output_file) {
            eprintln!("Error: the filename '{}' appears in the output list multiple times.", output_file);
            exit(1);
        }
        case_sensitive_output_set.insert(output_file);
    }

    // Sanity check - verify that the output filenames are case-insensitively unique.
    let mut case_insensitive_output_set = HashSet::new();
    for output_file in output_files.iter().filter(|s| !s.starts_with("#")).map(|s| s.to_lowercase()) {
        if case_insensitive_output_set.contains(&output_file) {
            eprintln!(
                "Error: the filename '{}' appears multiple times in the output list (case \
                insensitively). This may be intentional but Vimv always treats this situation \
                as an error to avoid accidentally overwriting files on case-insensitive \
                file systems.",
                output_file
            );
            exit(1);
        }
        case_insensitive_output_set.insert(output_file);
    }

    // List of files to delete.
    let mut delete_list: Vec<&str> = Vec::new();

    // List of rename operations as (src, dst) tuples.
    let mut rename_list: Vec<(String, String)> = Vec::new();

    // Set of input files to be renamed. Used to check for cycles.
    let mut rename_set: HashSet<String> = HashSet::new();

    // Populate the task lists.
    for (input_file, output_file) in input_files.iter().zip(output_files.iter()) {
        if input_file == output_file {
            continue;
        }

        if Path::new(output_file).is_dir() {
            if input_files.contains(output_file) {
                rename_list.push((input_file.to_string(), output_file.to_string()));
                rename_set.insert(input_file.to_string());
                continue;
            }
            eprintln!("Error: cannot overwrite the existing directory '{}'.", output_file);
            exit(1);
        }

        if output_file.starts_with("#") {
            delete_list.push(input_file);
            continue;
        }

        if Path::new(output_file).is_file() {
            if input_files.contains(output_file) {
                rename_list.push((input_file.to_string(), output_file.to_string()));
                rename_set.insert(input_file.to_string());
                continue;
            }

           if parser.found("force") {
                rename_list.push((input_file.to_string(), output_file.to_string()));
                rename_set.insert(input_file.to_string());
                continue;
            }

            eprintln!(
                "Error: the output file '{}' already exists, use --force to overwrite it.",
                output_file
            );
            exit(1);
        }

        rename_list.push((input_file.to_string(), output_file.to_string()));
        rename_set.insert(input_file.to_string());
    }

    // Check for cycles. If we find [src] being renamed to [dst] where [dst] is an input file that
    // hasn't yet been deleted or renamed, we rename [src] to [tmp] instead and later rename [tmp]
    // to [dst].
    for i in 0..rename_list.len() {
        if rename_set.contains(&rename_list[i].1) {
            let temp_file = get_temp_filename(&rename_list[i].0);
            rename_list.push((temp_file.clone(), rename_list[i].1.clone()));
            rename_list[i].1 = temp_file
        }
        rename_set.remove(&rename_list[i].0);
    }

    // Deletion loop. We haven't made any changes to the file system up to this point.
    for input_file in delete_list {
        delete_file(input_file, parser.found("quiet"));
    }

    // Rename loop.
    for (input_file, output_file) in rename_list {
        move_file(&input_file, &output_file, parser.found("quiet"));
    }
}


// Generate a unique temporary filename.
fn get_temp_filename(base: &str) -> String {
    let mut rng = rand::thread_rng();
    for _ in 0..10 {
        let candidate = format!("{}.vimv_temp_{:04}", base, rng.gen_range(0..10_000));
        if !Path::new(&candidate).exists() {
            return candidate;
        }
    }
    eprintln!(
        "Error: failed to generate a unique temporary filename of the form '{}.vimv_temp_XXXX'.",
        base
    );
    exit(1);
}


// Move the specified file to the system's trash/recycle bin.
fn delete_file(input_file: &str, quiet: bool) {
    if !quiet {
        println!("{} {}", "Deleting".green().bold(), input_file);
    }
    if let Err(err) = trash::delete(input_file) {
        eprintln!("Error: cannot delete the file '{}': {}", input_file, err);
        exit(1);
    }
}


// Rename `input_file` to `output_file`.
fn move_file(input_file: &str, output_file: &str, quiet: bool) {
    if !quiet {
        println!("{} {}", "Renaming".green().bold(), input_file);
        println!("      {}  {}", "⮑".green().bold(), output_file);
    }
    if let Some(parent_path) = Path::new(output_file).parent() {
        if !parent_path.is_dir() {
            if let Err(err) = std::fs::create_dir_all(parent_path) {
                eprintln!("Error: cannot create the required directory '{}': {}", parent_path.display(), err);
                exit(1);
            }
        }
    }
    if let Err(err) = std::fs::rename(input_file, output_file) {
        eprintln!("Error: cannot rename the file '{}' to '{}': {}", input_file, output_file, err);
        exit(1);
    }
}
