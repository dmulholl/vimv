use arguably::ArgParser;
use std::path::Path;
use std::process::exit;
use std::env;
use std::collections::HashSet;
use rand::Rng;
use std::process::Command;
use std::io::Read;


const HELP: &str = "
Usage: vimv [files]

  This utility lets you batch rename files using a text editor. Files to be
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

  You can delete a file by 'renaming' it to a blank line, but only if the
  --delete flag has been specified. Deleted files are moved to the system's
  trash/recycle bin.

  If the --git flag is specified then git will be used to rename or delete
  any files which are being tracked by git. (Note that this bypasses the
  recycle bin for deleted files.)

Arguments:
  [files]                   List of files to rename.

Options:
  -e, --editor <name>       Specify the editor to use.

Flags:
  -d, --delete              Enable file deletion.
  -f, --force               Overwrite existing files.
  -g, --git                 Use git for git-tracked files.
  -h, --help                Print this help text and exit.
  -q, --quiet               Only report errors.
  -v, --version             Print the version number and exit.
";


fn main() {
    let mut parser = ArgParser::new()
        .helptext(HELP)
        .version(env!("CARGO_PKG_VERSION"))
        .flag("force f")
        .flag("delete d")
        .flag("git g")
        .flag("quiet q")
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
    let input_files: Vec<String> = if parser.args.len() > 0 {
        parser.args.iter().map(|s| s.trim().to_string()).collect()
    } else {
        let mut buffer = String::new();
        if let Err(err) = std::io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error: cannot read filenames from standard input.");
            eprintln!("The OS reports: {}", err);
            exit(1);
        } else {
            buffer.lines().map(|s| s.trim().to_string()).collect()
        }
    };

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

    // Fetch the string of output filenames from the editor.
    let editor_input = input_files.join("\n");
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
    let mut cs_output_set = HashSet::new();
    for output_file in output_files.iter().filter(|s| !s.is_empty()) {
        if cs_output_set.contains(output_file) {
            eprintln!("Error: the filename '{}' appears in the output list multiple times.", output_file);
            exit(1);
        }
        cs_output_set.insert(output_file);
    }

    // Sanity check - verify that the (non-empty) output filenames are case-insensitively unique.
    let mut ci_output_set = HashSet::new();
    for output_file in output_files.iter().filter(|s| !s.is_empty()).map(|s| s.to_lowercase()) {
        if ci_output_set.contains(&output_file) {
            eprintln!(
                "Error: the filename '{}' appears multiple times in the output list (case \
                insensitively). This may be intentional but Vimv always treats this situation \
                as an error to avoid accidentally overwriting files on case-insensitive \
                file systems.",
                output_file
            );
            exit(1);
        }
        ci_output_set.insert(output_file);
    }

    // List of files to delete.
    let mut delete_list: Vec<&str> = Vec::new();

    // List of rename operations as (src, dst) tuples.
    let mut rename_list: Vec<(String, String)> = Vec::new();

    // Set of input files to be renamed. Used to check for cycles.
    let mut rename_set: HashSet<String> = HashSet::new();

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
                rename_set.insert(input_file.to_string());
            } else if parser.found("force") {
                rename_list.push((input_file.to_string(), output_file.to_string()));
                rename_set.insert(input_file.to_string());
            } else {
                eprintln!(
                    "Error: the output file '{}' already exists, use --force to overwrite it.",
                    output_file
                );
                exit(1);
            }
        } else {
            rename_list.push((input_file.to_string(), output_file.to_string()));
            rename_set.insert(input_file.to_string());
        }
    }

    // Check for cycles. If we find src being renamed to dst where dst is an input file that hasn't
    // yet been deleted or renamed, we rename src to tmp instead and later rename tmp to dst.
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
        delete_file(input_file, parser.found("git"), parser.found("quiet"));
    }

    // Rename loop.
    for (input_file, output_file) in rename_list {
        move_file(&input_file, &output_file, parser.found("git"), parser.found("quiet"));
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
    eprintln!("Error: cannot generate a temporary filename of the form '{}.vimv_temp_XXXX'.", base);
    exit(1);
}


// Delete the specified file using 'git rm' or move it to the system's trash/recycle bin.
fn delete_file(input_file: &str, use_git: bool, quiet: bool) {
    if !quiet {
        println!("Deleting: {}", input_file);
    }
    if use_git && is_git_tracked(input_file) {
        match Command::new("git").arg("rm").arg("-r").arg(input_file).output() {
            Err(err) => {
                eprintln!("Error: cannot 'git rm' the file '{}'.", input_file);
                eprintln!("The OS reports: {}", err);
                exit(1);
            },
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("Error: cannot 'git rm' the file '{}'.", input_file);
                    eprintln!("Git reports: {}", String::from_utf8_lossy(&output.stderr).trim());
                    exit(1);
                }
            }
        };
    } else if let Err(err) = trash::delete(input_file) {
        eprintln!("Error: cannot delete the file '{}'.", input_file);
        eprintln!("The OS reports: {}", err);
        exit(1);
    }
}


// Rename `input_file` to `output_file`.
fn move_file(input_file: &str, output_file: &str, use_git: bool, quiet: bool) {
    if !quiet {
        println!("Renaming: {} → {}", input_file, output_file);
    }
    if let Some(parent_path) = Path::new(output_file).parent() {
        if !parent_path.is_dir() {
            if let Err(err) = std::fs::create_dir_all(parent_path) {
                eprintln!("Error: cannot create the required directory '{}'.", parent_path.display());
                eprintln!("The OS reports: {}", err);
                exit(1);
            }
        }
    }
    if use_git && is_git_tracked(input_file) {
        match Command::new("git").arg("mv").arg("-f").arg(input_file).arg(output_file).output() {
            Err(err) => {
                eprintln!("Error: cannot 'git mv' the file '{}' to '{}'.", input_file, output_file);
                eprintln!("The OS reports: {}", err);
                exit(1);
            },
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("Error: cannot 'git mv' the file '{}' to '{}'.", input_file, output_file);
                    eprintln!("Git reports: {}", String::from_utf8_lossy(&output.stderr).trim());
                    exit(1);
                }
            }
        };
    } else if let Err(err) = std::fs::rename(input_file, output_file) {
        eprintln!("Error: cannot rename the file '{}' to '{}'.", input_file, output_file);
        eprintln!("The OS reports: {}", err);
        exit(1);
    }
}


// Returns true if the file is being tracked by git.
fn is_git_tracked(file: &str) -> bool {
    match Command::new("git").arg("ls-files").arg("--error-unmatch").arg(file).output() {
        Err(err) => {
            eprintln!("Error: cannot check if the file '{}' is being tracked by git.", file);
            eprintln!("The OS reports: {}", err);
            exit(1);
        },
        Ok(output) => {
            output.status.success()
        }
    }
}
