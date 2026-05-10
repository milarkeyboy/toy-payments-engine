//! Command line interface for the payment engine.

use super::engine::Engine;
use super::requests::{Amount, ClientId, Request, RequestType, TransactionId};

use serde::Deserialize;

use std::error::Error;
use std::fmt;
use std::path::Path;

/// Wraps the engine in a command line interface, and feeds payment requests
/// from its inputs.
pub struct Cli {
    engine: Engine,
}

/// Possible errors from running the CLI.
#[derive(Debug)]
pub enum CliError {
    /// An input request used an unknown type of transaction.
    UnhandledTransactionType,

    /// An input request was missing an amount value.
    MissingAmount,

    /// A transaction attempted to use an invalid amount value.
    InvalidAmount,
}

impl Cli {
    /// Initialise the CLI for the given engine instance.
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    /// Run the CLI, feeding input from a CSV file, and outputting the final
    /// statement to stdout.
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // We could have had something more sophisticated with a help message
        // and so on. But for the purposes of this CLI, simple is fine enough.
        let file_path_str = std::env::args()
            .nth(1)
            .expect("Engine expects a single CSV file as input");
        let csv_file = Path::new(&file_path_str);

        let mut reader = csv::ReaderBuilder::new()
            // Allow sprinklings of whitespace.
            .trim(csv::Trim::All)
            .from_path(csv_file)?;

        // Iterate over lines from the CSV source and pass them onto the engine
        // to action.
        for line in reader.deserialize() {
            let request: CsvRequest = line?;

            let process_amount = || -> Result<Amount, Box<CliError>> {
                if let Some(value) = request.amount.as_deref() {
                    return value.parse().map_err(|_| Box::new(CliError::InvalidAmount));
                }

                Err(Box::new(CliError::MissingAmount))
            };

            let request = match request.r#type.as_str() {
                "deposit" => Request {
                    client: ClientId(request.client),
                    transaction: TransactionId(request.tx),
                    request_type: RequestType::Deposit {
                        amount: process_amount()?,
                    },
                },
                "withdrawal" => Request {
                    client: ClientId(request.client),
                    transaction: TransactionId(request.tx),
                    request_type: RequestType::Withdrawal {
                        amount: process_amount()?,
                    },
                },
                "dispute" => Request {
                    client: ClientId(request.client),
                    transaction: TransactionId(request.tx),
                    request_type: RequestType::Dispute,
                },
                "resolve" => Request {
                    client: ClientId(request.client),
                    transaction: TransactionId(request.tx),
                    request_type: RequestType::Resolve,
                },
                "chargeback" => Request {
                    client: ClientId(request.client),
                    transaction: TransactionId(request.tx),
                    request_type: RequestType::Chargeback,
                },
                &_ => return Err(Box::new(CliError::UnhandledTransactionType)),
            };

            self.engine.action(request);
        }

        // Sorting the statement in ascending client IDs is not required, but
        // given this is a CLI it will make it easier to read for humans.
        let mut statement = self.engine.generate_statement();
        statement.sort_by_key(|r| r.client);

        // Write the statement as output.
        let mut writer = csv::Writer::from_writer(std::io::stdout());
        for statement_line in statement {
            writer.serialize(statement_line)?;
        }
        writer.flush()?;

        Ok(())
    }
}

// Sadly, using serde's internally tagged enum mechanism does not
// work with CSV:
//
// https://github.com/BurntSushi/rust-csv/issues/211
//
// Even the PR to add the warning to the documentation is 5 years
// old. That would have saved me some time :(
//
// So, the solution is for this module to handle deserialisation from CSV. The
// type consumed by the engine would still work deserialised from other forms
// (e.g. JSON over TCP).
#[derive(Debug, Deserialize)]
struct CsvRequest {
    r#type: String,
    client: u16,
    tx: u32,

    #[serde(default)]
    amount: Option<String>,
}

// Custom error boilerplate:

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to serve CSV data from the CLI")
    }
}

impl Error for CliError {}
