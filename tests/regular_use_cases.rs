//! Tests that run through standard use cases.

mod helpers;
use helpers::{ExpectedOutput, run};

// Money goes in, money goes out...
#[test]
fn deposits_and_withdrawals() {
    run(
        "
type,client,tx,amount
deposit,1,1,1.0
deposit,2,2,2.0
deposit,1,3,2.0
withdrawal,1,4,1.5
withdrawal,2,5,0.5
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,1.5,0,1.5,false
2,1.5,0,1.5,false
    ",
        ),
    );
}

// Decimal arithmetic should be exact, not subject to binary float rounding.
#[test]
fn fixed_point_arithmetic() {
    run(
        "
type,client,tx,amount
deposit,1,1,0.3
withdrawal,1,2,0.1
withdrawal,1,3,0.2
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,0,0,0,false
    ",
        ),
    );
}

// Basic dispute scenario.
#[test]
fn disputes() {
    run(
        "
type,client,tx,amount
deposit,1,1,6
deposit,7,2,3
dispute,1,1,
deposit,1,3,3.2
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,3.2,6,9.2,false
7,3,0,3,false
    ",
        ),
    );
}

// Basic resolve scenario.
#[test]
fn resolutions() {
    run(
        "
type,client,tx,amount
deposit,5,9,20
deposit,5,3,15
dispute,5,3,
withdrawal,5,7,30
resolve,5,3,
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
5,35,0,35,false
    ",
        ),
    );
}

// Basic chargeback scenario.
#[test]
fn chargebacks() {
    run(
        "
type,client,tx,amount
deposit,3,0,10
dispute,3,0,
deposit,10,99,4
deposit,3,777,5
chargeback,3,0,
withdrawal,3,64,2
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
3,5,0,5,true
10,4,0,4,false
    ",
        ),
    );
}

// Locked accounts don't allow further deposits or withdrawals.
#[test]
fn locked_account() {
    run(
        "
type,client,tx,amount
deposit,0,4,50
dispute,0,4,
chargeback,0,4,
deposit,0,2,20
withdrawal,0,47,25
deposit,0,7,105
withdrawal,0,88,999
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
0,0,0,0,true
    ",
        ),
    );
}

// Inputs should support up to 4 decimal places, and output should use the same
// limit without unnecessary trailing zeroes.
#[test]
fn output_decimal_places() {
    run(
        "
type,client,tx,amount
deposit,1,1,1.1234
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,1.1234,0,1.1234,false
    ",
        ),
    );

    run(
        "
type,client,tx,amount
deposit,1,1,1.1235
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,1.1235,0,1.1235,false
    ",
        ),
    );

    // At the boundary right before we would start to lose precision with 64-bit
    // floating point
    run(
        "
type,client,tx,amount
deposit,1,1,1000000000000.1234
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,1000000000000.1234,0,1000000000000.1234,false
    ",
        ),
    );

    // I mean, if someone has this much money, they can probably afford a better
    // payments engine. But fixed-point storage still preserves the supported
    // decimal places exactly.
    run(
        "
type,client,tx,amount
deposit,1,1,10000000000000.1234
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,10000000000000.1234,0,10000000000000.1234,false
    ",
        ),
    );
}
