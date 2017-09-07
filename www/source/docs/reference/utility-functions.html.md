---
title: Utility functions
---

# Utility functions
The following helper functions can be useful in your plan to help you build your package correctly. They are mostly used for debugging and building packages.

**attach()**
: Attaches your script to an interactive debugging session, which lets you check the state of variables, call arbitrary functions, and turn on higher levels of logging by using the `set -x` command and switch.

  To use attach, add `attach` to any callback or part of your plan.sh file and the debugging session with start up when hab-plan-build comes to that part in the file.

**download_file()**
: Downloads a file from a source URL to a local file and uses an optional
shasum to determine if an existing file can be used.

  If an existing file is present and the third argument is set with a shasum
digest, the file will be checked to see if it's valid. If so, the function
ends early and returns 0. Otherwise, the shasums do not match so the
file-on-disk is removed and a normal download proceeds as though no previous
file existed. This is designed to restart an interrupted download.

Any valid `wget` URL will work.

Downloads every time, even if the file exists locally:

~~~
download_file http://example.com/file.tar.gz file.tar.gz
~~~

Downloads if no local file is found:

~~~
download_file http://example.com/file.tar.gz file.tar.gz abc123...
~~~

File matches checksum: download is skipped, local file is used:

~~~
download_file http://example.com/file.tar.gz file.tar.gz abc123...
~~~

File doesn't match checksum: local file removed, download attempted:

~~~
download_file http://example.com/file.tar.gz file.tar.gz ohnoes...
~~~

Will return 0 if a file was downloaded or if a valid cached file was found.

**pkg\_path\_for()**
: Returns the path for a build or runtime package dependency on stdout from the list of dependencies referenced in `pkg_deps` or `pkg_build_deps`. This is useful if you need to install or reference specific dependencies from within a callback, such as `do_build()` or `do_install()`.

  Here's an example of how to use this function to retrieve the path to the perl binary in the core/perl package:

  ~~~
  _perl_path="$(pkg_path_for core/perl)/bin/perl"
  ~~~

**fix_interpreter()**
: Edits the `#!` shebang of the target file in-place. This is useful for changing hardcoded paths defined by your source files to the equivalent path in a Habitat package. You must include the required package that provides the expected path for the shebang in pkg_deps. This function performs a greedy match against the specified interpreter in the target file(s).

  To use this function in your plan, you must specify the following arguments:
    1. The target file or files
    2. The name of the package that contains the interpreter
    3. The relative directory and binary path to the interpreter

  For example, to replace all the files in `node_modules/.bin` that have `#!/usr/bin/env` with the coreutils path to `bin/env` (/hab/pkgs/core/coreutils/8.24/20160219013458/bin/env), you must quote the wildcard target as shown below.

  ~~~
  fix_interpreter "node_modules/.bin/*" core/coreutils bin/env
  ~~~

  For a single target, reference the file directly:

  ~~~
  fix_interpreter node_modules/.bin/concurrent core/coreutils bin/env
  ~~~

**pkg\_interpreter\_for()**
: Returns the path for the given package and interpreter by reading it from the INTERPRETERS metadata in the package. The directory of the interpreter needs to be specified, as an interpreter binary might live in `bin`, `sbin`, or `libexec`, depending on the software.

  The following shows how to call pkg_interpreter_for with the package and interpreter arguments specified.

  ~~~
  pkg_interpreter_for core/coreutils bin/env
  ~~~

  This function will return 0 if the specified package and interpreter were found, and 1 if the package could not be found or the interpreter is not specified for that package.

**pkg_version()**
: An optional way to determine the value for `$pkg_version`. The function must print the computed version string to standard output and will be called when the Plan author invokes the `update_pkg_version()` helper.

**update\_pkg\_version()**
: Updates the value for `$pkg_version` by calling a Plan author-provided `pkg_version()` function. This function must be explicitly called in a Plan in or after the `do_before()` build phase but before the `do_prepare()` build phase. The `$pkg_version` variable will be updated and any other relevant variables will be recomputed. The following examples show how to use `pkg_version()` and `update_pkg_version()` together.

This plan concatenates a static file in the source root of the
project to determine the version in the
`do_before()` phase:

~~~
pkg_version() {
  cat "$SRC_PATH/version.txt"
}

do_before() {
  do_default_before
  update_pkg_version
}
~~~

The `pkg_version()` function in this plan uses grep and sed before using the
date binary to format the final version string to standard output. As
the downloaded file is required before running the version logic, the
`update_pkg_version()` helper is called in the `do_download()` build
phase:

~~~
pkg_version() {
  local build_date

  # Extract the build date of the certificates file
  build_date=$(cat $HAB_CACHE_SRC_PATH/$pkg_filename \
    | grep 'Certificate data from Mozilla' \
    | sed 's/^## Certificate data from Mozilla as of: //')

  date --date="$build_date" "+%Y.%m.%d"
}

do_download() {
  do_default_download
  update_pkg_version
}
~~~

**abspath()**
: Return the absolute path for a path, which might be absolute or relative.

**exists()**
: Checks that the command exists. Returns 0 if it does, 1 if it does not.

**build_line()**
: Print a line of build output. Takes a string as its only argument.

~~~
build_line "Checksum verified - ${pkg_shasum}"
~~~

**warn()**
: Print a warning line on stderr. Takes a string as its only argument.

~~~
warn "Checksum failed"
~~~

**debug()**
: Prints a line only if the `$DEBUG` environment value is set to 1. The `debug` function takes a string as its only argument.

~~~
DEBUG=1
debug "Only if things are set"
~~~

**exit_with()**
: Exits the program with an error message and a status code.

~~~
exit_with "Something bad happened" 55
~~~

**trim()**
: Trims leading and trailing whitespace characters from a bash variable.

**record()**
: Takes a session name and command to run as arguments function appends a timestamp to the log file. Alternative to piping build through tee.

~~~
Usage: record <SESSION> [CMD [ARG ...]]
record mysoftware build /src/mysoftware
~~~
