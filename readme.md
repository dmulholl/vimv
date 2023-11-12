# Vimv

[1]: https://www.dmulholl.com/dev/vimv.html
[2]: https://crates.io/crates/vimv



This simple command line utility lets you batch-rename files from the comfort of your favourite text editor. You specify the files to be renamed as arguments, e.g.

    $ vimv *.mp3

The list of files will be opened in the editor specified by the `$EDITOR` environment variable, one filename per line. Edit the list, save, and exit. The files will be renamed to the edited filenames.



### Installation

Vimv is written in Rust &mdash; if you have a Rust compiler available, you can install it directly from the package index using `cargo`:

    $ cargo install vimv

* [Documentation][1]
* [Package][2]



### Interface

Run `vimv --help` to view the command line help:

    Usage: vimv [files]

      This utility lets you batch-rename files using a text editor.
      Files to be renamed should be supplied as a list of command-line
      arguments, e.g.

        $ vimv *.mp3

      The list of files will be opened in the editor specified by the
      $EDITOR environment variable, one filename per line. Edit the
      list, save, and exit. The files will be renamed to the edited
      filenames. Directories along the renamed paths will be created
      as required.

      If the input file list is empty, Vimv defaults to listing the
      contents of the current working directory.

      Vimv supports cycle-renaming. You can safely rename A to B, B to
      C, and C to A in a single operation.

      Use the --force flag to overwrite existing files that aren't part
      of a renaming cycle. (Existing directories are never overwritten.
      If you attempt to overwrite a directory, the program will exit with
      an error message and a non-zero status code.)

      You can delete a file or directory by prefixing its name with a
      '#' symbol. Deleted files and directories are moved to the system's
      trash/recycle bin.

    Arguments:
      [files]                   List of files to rename.

    Options:
      -e, --editor <name>       Specify the editor to use.

    Flags:
      -f, --force               Overwrite existing files.
      -h, --help                Print this help text.
      -q, --quiet               Quiet mode -- only report errors.
      -s, --stdin               Read the list of input files from stdin.
      -v, --version             Print the version number.

Vimv simply ignores any filenames that haven't been changed so you don't have to be overly fussy
about specifying its input. You can run:

    $ vimv *

to get a full listing of a directory's contents, change just the items you want, and Vimv will
ignore the rest.



### Cycle Renaming

Vimv supports cycle-renaming. You can safely rename A to B, B to C, and C to A in a single operation.



### Deleting Files

You can delete a file or directory by prefixing its name with a `#` symbol.
Deleted files and directories are moved to the system's trash/recycle bin.



### Graphical Editors

If you want to use a graphical editor like VS Code or Sublime Text instead of a terminal editor like Vim then (depending on your operating system) you may need to add a 'wait' flag to the `$EDITOR` variable to force the editor to block, e.g.

    EDITOR="code -w"      # for VS Code
    EDITOR="subl -w"      # for Sublime Text
    EDITOR="atom -w"      # for Atom

The same flag can be used with the `--editor` option, e.g.

    $ vimv *.mp3 --editor "code -w"



### Piped Input

You can pipe a list of filenames into Vimv from a tool like `ls` or `fd`, e.g.

    $ fd .txt | vimv --stdin

Note that your editor may not appreciate inheriting a standard input stream that's connected to a pipe rather than a terminal.
Graphical editors tend to handle this situation without complaint, as does Neovim in the terminal.
Vim prints a warning, then works, then borks your terminal session until you run `reset`. YMMV.

(Because of this inconsistent behaviour, this feature is hidden behind a `--stdin/-s` flag.)



### License

Zero-Clause BSD (0BSD).
