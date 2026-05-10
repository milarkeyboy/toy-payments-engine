//! Tests that check input validation and sanitisation.

mod helpers;
use helpers::{ExpectedOutput, run};

use assert_cmd::Command;

#[test]
fn bad_file() {
    // No file argument is a failure.
    Command::cargo_bin("toy-payments-engine")
        .unwrap()
        .assert()
        .failure();

    // Non-existent input files are failures.
    Command::cargo_bin("toy-payments-engine")
        .unwrap()
        .arg("/does/not/exist")
        .assert()
        .failure();
}

#[test]
fn empty_input() {
    // Valid header with no data is valid CSV, but won't produce any meaningful
    // output.
    run(
        "
type,client,tx,amount
    ",
        ExpectedOutput::Success(""),
    );
}

#[test]
fn missing_values() {
    // Missing values in the CSV lines is malformed input => failure
    run(
        "
type,client,tx,amount
,,,
    ",
        ExpectedOutput::Failure,
    );
}

#[test]
fn extra_whitespace() {
    // Whitespace in and around the CSV lines is valid input.
    run(
        "
type,client,tx,amount
           deposit,1         ,1,  1.0
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,1,0,1,false
    ",
        ),
    );
}

// Checks for invalid float values and treats them as an input contract
// violation.
#[test]
fn invalid_floats() {
    run(
        "
type,client,tx,amount
deposit,1,1,NaN
    ",
        ExpectedOutput::Failure,
    );

    // As much as we'd all love infinite cash, it's sadly not possible :'(
    run(
        "
type,client,tx,amount
deposit,1,1,inf
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
deposit,1,1,+inf
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
deposit,1,1,-inf
    ",
        ExpectedOutput::Failure,
    );
}

// Payment amounts must be positive.
#[test]
fn non_positive_amounts() {
    run(
        "
type,client,tx,amount
deposit,1,1,-10
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
withdrawal,1,1,-10
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
deposit,1,1,0
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
withdrawal,1,1,0.0000
    ",
        ExpectedOutput::Failure,
    );
}

// Input precision is limited to ten-thousandths.
#[test]
fn too_many_decimal_places() {
    run(
        "
type,client,tx,amount
deposit,1,1,1.12345
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
withdrawal,1,1,0.00001
    ",
        ExpectedOutput::Failure,
    );
}

// Integer values outside the bounds of the required size.
#[test]
fn out_of_bound_integers() {
    // Client is u16, so go above and below that.
    run(
        "
type,client,tx,amount
deposit,100000,1,1
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
deposit,-5,1,1
    ",
        ExpectedOutput::Failure,
    );

    // tx is u32, so do the same
    run(
        "
type,client,tx,amount
deposit,1,5000000000,1
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
deposit,1,-1,1
    ",
        ExpectedOutput::Failure,
    );
}

// Checks we handle invalid transaction type strings.
#[test]
fn invalid_transaction_types() {
    run(
        "
type,client,tx,amount
food,1,1,1
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
(*%^&#$#%#,1,1,1
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAARG,1,1,1
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
1234,1,1,1
    ",
        ExpectedOutput::Failure,
    );
    run(
        "
type,client,tx,amount
depositt,1,1,1
    ",
        ExpectedOutput::Failure,
    );
}
