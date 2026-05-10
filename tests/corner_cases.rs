//! Tests that run exceptional or unusual use cases.

mod helpers;
use helpers::{ExpectedOutput, run};

// Check that identical transactions are only applied once.
#[test]
fn idempotent() {
    run(
        "
type,client,tx,amount
deposit,3,2,1
deposit,3,2,1
deposit,3,2,1
deposit,3,3,1
withdrawal,3,1,1
deposit,3,2,1
deposit,3,5,1
deposit,3,2,1
withdrawal,3,1,1
withdrawal,3,1,1
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
3,2,0,2,false
    ",
        ),
    );
}

// Resolutions for undisputed transactions have no effect.
#[test]
fn undisputed_resolutions() {
    run(
        "
type,client,tx,amount
deposit,1,1,10
deposit,2,2,5
resolve,1,1,
withdrawal,1,3,5
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,5,0,5,false
2,5,0,5,false
    ",
        ),
    );
}

// Chargebacks for undisputed transactions have no effect.
#[test]
fn undisputed_chargebacks() {
    run(
        "
type,client,tx,amount
deposit,1,1,10
deposit,2,2,5
chargeback,1,1,
withdrawal,1,3,5
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,5,0,5,false
2,5,0,5,false
    ",
        ),
    );
}

// Ignored dispute, resolve, and chargeback requests must not create accounts.
#[test]
fn ignored_requests_do_not_create_accounts() {
    run(
        "
type,client,tx,amount
dispute,9,42,
resolve,10,42,
chargeback,11,42,
    ",
        ExpectedOutput::Success(""),
    );
}

// Requests against another client's transaction must not create accounts.
#[test]
fn wrong_client_requests_do_not_create_accounts() {
    run(
        "
type,client,tx,amount
deposit,1,1,10
dispute,2,1,
resolve,3,1,
chargeback,4,1,
withdrawal,1,2,5
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,5,0,5,false
    ",
        ),
    );
}

// Duplicate transaction IDs from new clients must not create accounts.
#[test]
fn duplicate_transactions_do_not_create_accounts() {
    run(
        "
type,client,tx,amount
deposit,1,1,5
deposit,2,1,5
withdrawal,3,1,1
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,5,0,5,false
    ",
        ),
    );
}

// Disputes on non-existent transactions have no effect.
#[test]
fn disputing_non_existent_transaction() {
    run(
        "
type,client,tx,amount
deposit,1,1,10
deposit,2,2,5
dispute,1,3,
deposit,1,3,5
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,15,0,15,false
2,5,0,5,false
    ",
        ),
    );
}

// Chargebacks on non-existent transactions have no effect.
#[test]
fn chargeback_on_non_existent_transaction() {
    run(
        "
type,client,tx,amount
deposit,1,1,10
deposit,2,2,5
chargeback,1,3,
deposit,1,3,5
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,15,0,15,false
2,5,0,5,false
    ",
        ),
    );
}

// Subsequent conflicting transactions that share the same ID as ones already
// actioned have no effect.
#[test]
fn conflicting_transactions() {
    run(
        "
type,client,tx,amount
deposit,1,1,5
deposit,1,1,2
withdrawal,1,2,3
withdrawal,1,1,2
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,2,0,2,false
    ",
        ),
    );
}

// If an account is locked, transactions in its history can continue to be
// disputed to account for many instances of fraudulent or erroneous scenarios.
#[test]
fn dispute_locked_account() {
    run(
        "
type,client,tx,amount
deposit,1,1,10
deposit,1,2,5
deposit,1,3,7
dispute,1,1,
chargeback,1,1,
dispute,1,2,
dispute,1,3,
chargeback,1,3,
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,0,5,5,true
    ",
        ),
    );
}

// Attempting to resolve a transaction which has already been charged back will
// have no effect. The chargeback is treated as something going wrong and
// needing human intervention.
#[test]
fn resolving_a_chargeback() {
    run(
        "
type,client,tx,amount
deposit,1,1,7
deposit,1,2,5
dispute,1,1,
chargeback,1,1,
dispute,1,1,
resolve,1,1,
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,5,0,5,true
    ",
        ),
    );
}

// The inverse of the above: charging back a previously resolved transaction
// will result in the chargeback succeeding. Again, the locking of the account
// is critical if we believe anything has possibly gone wrong.
#[test]
fn charging_back_a_resolved_transaction() {
    run(
        "
type,client,tx,amount
deposit,1,1,9
deposit,1,2,6
dispute,1,1,
resolve,1,1,
dispute,1,1,
resolve,1,1,
dispute,1,1,
chargeback,1,1,
dispute,1,1,
resolve,1,1,
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,6,0,6,true
    ",
        ),
    );
}

// Balances can go negative when chargebacks occur. It is then up to the
// business as to how to handle this situation.
#[test]
fn negative_balance_after_chargeback() {
    run(
        "
type,client,tx,amount
deposit,1,1,5
deposit,1,2,2
withdrawal,1,3,6
dispute,1,1,
chargeback,1,1,
dispute,1,2,
chargeback,1,2,
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,-6,0,-6,true
    ",
        ),
    );
}

// Disputes are only handled for deposits.
#[test]
fn disputing_withdrawal_does_nothing() {
    run(
        "
type,client,tx,amount
deposit,1,1,5
withdrawal,1,2,3
dispute,1,2,
    ",
        ExpectedOutput::Success(
            "
client,available,held,total,locked
1,2,0,2,false
    ",
        ),
    );
}
