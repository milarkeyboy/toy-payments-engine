//! Set of helpers for writing integration tests. Such tests are intended to
//! run the CLI binary, and make assertions on the output in stdout.

// Since each integration tests is built as its own crate, we have to deal
// with dead code warnings in shared helpers such as these.
#![allow(dead_code)]

use assert_cmd::Command;
use std::io::Write;
use tempfile::NamedTempFile;

/// The expected outcome of a run of the engine binary.
pub enum ExpectedOutput {
    /// The binary exited with a non-zero exit code.
    Failure,

    /// The binary exited successfully, and a string value is expected as the
    /// stdout.
    Success(&'static str),
}

/// Run the CLI and assert the expected output. For successes, the expected
/// output string will be trimmed to make sure whitespaces don't cause
/// unnecessary false negatives.
pub fn run(input: &str, output: ExpectedOutput) {
    // Strings are trimmed to allow tests to use newlines liberally for
    // readability.
    let mut input_file = NamedTempFile::new().unwrap();
    write!(input_file, "{}", input.trim()).unwrap();

    let mut command = Command::cargo_bin("toy-payments-engine").unwrap();
    let result = command.arg(input_file.path()).assert();

    match output {
        ExpectedOutput::Failure => {
            result.failure();
        }
        ExpectedOutput::Success(expected_out) => {
            // Make sure each line matches what was expected, but ignore leading
            // and trailing newlines.
            let received_out =
                String::from_utf8(result.success().get_output().stdout.clone()).unwrap();
            assert_eq!(expected_out.trim(), received_out.trim());
        }
    };
}
