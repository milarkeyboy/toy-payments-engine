//! The core module of the payment engine.

use super::requests::{Amount, ClientId, Request, RequestType, TransactionId};

use serde::{Serialize, Serializer};

use std::collections::HashMap;

/// The primary transaction request processor in the payment engine. Handles
/// requests from arbitrary sources, and manages client accounts internally to
/// produce a final statement.
pub struct Engine {
    // Hashmaps are used here to allow fast lookup of data to either mutate
    // or amend. It is assumed that duplicating a hash is very unlikely, and
    // thus is not a performance hindrance. Resizing may prove to be a
    // performance problem, but that would need to be benchmarked against real
    // data.
    accounts: HashMap<ClientId, Account>,
    transaction_history: HashMap<TransactionId, Transaction>,
}

/// Line item in a generated statement from the Engine.
#[derive(Serialize)]
pub struct StatementItem {
    /// The client to which this line item pertains.
    pub client: ClientId,

    /// How many funds are available to the client to use.
    #[serde(serialize_with = "format_units")]
    pub available: i64,

    /// How many funds are currently being held in dispute by the credit
    /// provider.
    #[serde(serialize_with = "format_units")]
    pub held: i64,

    /// The total funds within the client's account.
    #[serde(serialize_with = "format_units")]
    pub total: i64,

    /// Whether the client's account has been locked as a result of a
    /// chargeback.
    pub locked: bool,
}

impl Engine {
    /// Create a new payment engine.
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transaction_history: HashMap::new(),
        }
    }

    /// Action a given request, processing any updates to client accounts.
    pub fn action(&mut self, request: Request) {
        // Handle each request type separately, applying the appropriate
        // business logic. Note that there is a fair amount of duplication in
        // each of the arms of this match statement. This is something that
        // could be improved later.
        match request.request_type {
            RequestType::Deposit { amount } => {
                if self.transaction_history.contains_key(&request.transaction) {
                    return;
                }

                let account = self
                    .accounts
                    .entry(request.client)
                    .or_insert(Account::new());

                // Using a frozen account is not allowed.
                if account.locked {
                    return;
                }

                // Deposit the funds and add to history.
                account.available += amount.units();
                self.transaction_history.insert(
                    request.transaction,
                    Transaction {
                        client: request.client,
                        transaction_type: TransactionType::Deposit {
                            amount,
                            state: DepositState::Actioned,
                        },
                    },
                );
            }

            RequestType::Withdrawal { amount } => {
                if self.transaction_history.contains_key(&request.transaction) {
                    return;
                }

                let Some(account) = self.accounts.get_mut(&request.client) else {
                    return;
                };

                // Using a frozen account is not allowed.
                if account.locked {
                    return;
                }

                if account.available < amount.units() {
                    // Cannot overdraw, and the transaction is dropped (not
                    // entered into history). Maybe depending on what kind of
                    // audit trails need to be left, logging this would be wise.
                    return;
                }

                // Withdraw the funds, and add to history.
                account.available -= amount.units();
                self.transaction_history.insert(
                    request.transaction,
                    Transaction {
                        client: request.client,
                        transaction_type: TransactionType::Withdrawal,
                    },
                );
            }

            RequestType::Dispute => {
                let Some(disputed) = self.transaction_history.get_mut(&request.transaction) else {
                    return;
                };

                if disputed.client != request.client {
                    // A request to dispute a transaction has a mismatching
                    // client ID. Assume not valid and drop.
                    return;
                }

                match &mut disputed.transaction_type {
                    TransactionType::Deposit { amount, state } => {
                        if let DepositState::Actioned = state {
                            let Some(account) = self.accounts.get_mut(&request.client) else {
                                return;
                            };

                            // Move funds from available to being held.
                            account.available -= amount.units();
                            account.held += amount.units();
                            *state = DepositState::InDispute;
                        }
                    }
                    // Withdrawals are not disputable (see README).
                    TransactionType::Withdrawal => {}
                }
            }

            RequestType::Resolve => {
                let Some(to_resolve) = self.transaction_history.get_mut(&request.transaction)
                else {
                    return;
                };

                if to_resolve.client != request.client {
                    return;
                }

                match &mut to_resolve.transaction_type {
                    TransactionType::Deposit { amount, state } => {
                        if let DepositState::InDispute = state {
                            let Some(account) = self.accounts.get_mut(&request.client) else {
                                return;
                            };

                            // Move funds from being held back to available.
                            account.held -= amount.units();
                            account.available += amount.units();
                            *state = DepositState::Actioned;
                        }
                    }
                    // Withdrawals are not disputable (see README).
                    TransactionType::Withdrawal => {}
                }
            }

            RequestType::Chargeback => {
                let Some(to_chargeback) = self.transaction_history.get_mut(&request.transaction)
                else {
                    return;
                };

                if to_chargeback.client != request.client {
                    return;
                }

                match &mut to_chargeback.transaction_type {
                    TransactionType::Deposit { amount, state } => {
                        if let DepositState::InDispute = state {
                            let Some(account) = self.accounts.get_mut(&request.client) else {
                                return;
                            };

                            // Issue chargeback by releasing that which was held.
                            account.held -= amount.units();
                            *state = DepositState::ChargedBack;

                            // With this issued, the account is now locked from
                            // further use.
                            account.locked = true;
                        }
                    }
                    // Withdrawals are not disputable (see README).
                    TransactionType::Withdrawal => {}
                }
            }
        }
    }

    /// Generate a statement based on the current state of all client accounts.
    /// This is intended to summarise the full set of clients the engine has
    /// handled requests for, and show the state of their funds.
    pub fn generate_statement(&self) -> Vec<StatementItem> {
        let mut statement: Vec<StatementItem> = vec![];
        for (client, account) in &self.accounts {
            statement.push(StatementItem {
                client: *client,
                available: account.available,
                held: account.held,
                total: account.total(),
                locked: account.locked,
            });
        }
        statement
    }
}

// POD-type representing a client's account.
struct Account {
    available: i64,
    held: i64,
    locked: bool,
}

// Even though the engine currently only cares about differentiating between
// "disputable" and "indisputable" transactions, we maintain the original
// terminology to cater for possible future extensions (e.g. making the
// transaction history viewable by the user).
enum TransactionType {
    Deposit { amount: Amount, state: DepositState },
    Withdrawal,
}

// A transaction that results in the moving of funds.
struct Transaction {
    client: ClientId,
    transaction_type: TransactionType,
}

// The possible states that a transaction can be in (since they're mutable).
enum DepositState {
    // The deposit was actioned by the engine, resulting in additional funds in
    // the users account.
    Actioned,

    // The deposit is currently in dispute. This can either be charged back or
    // resolved back to Actioned.
    InDispute,

    // The deposit has been charged back, and cannot be actioned or disputed.
    ChargedBack,
}

impl Account {
    // Create an empty account.
    fn new() -> Self {
        Self {
            available: 0,
            held: 0,
            locked: false,
        }
    }

    // Calculate the total funds within the account.
    fn total(&self) -> i64 {
        self.available + self.held
    }
}

// Format fixed-point values to max 4 decimal places. An argument could be made
// that this should live in the CLI module (since it pertains to the output
// interface), but that would require writing a whole serialisation method
// for the struct. Something to consider as an improvement.
fn format_units<S>(val: &i64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let absolute = i128::from(*val).abs();
    let whole = absolute / i128::from(Amount::SCALE);
    let fractional = absolute % i128::from(Amount::SCALE);

    let mut formatted = if fractional == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fractional:04}")
            .trim_end_matches('0')
            .to_string()
    };

    if *val < 0 {
        formatted.insert(0, '-');
    }

    s.serialize_str(&formatted)
}
