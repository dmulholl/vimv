# Vimv

This simple command line utility lets you batch-rename files from the comfort of your favourite text editor. You specify the files to be renamed as arguments, e.g.

    $ vimv *.mp3

The list of files will be opened in the editor specified by the `$EDITOR` environment variable, one filename per line. Edit the list, save, and exit. The files will be renamed to the edited filenames.

Vimv is written in Rust and can be installed directly from the package index using `cargo`:

    $ cargo install vimv

See the [documentation](http://www.dmulholl.com/dev/vimv.html) for details.
