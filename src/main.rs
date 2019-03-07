extern crate abci;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate arrayref;
extern crate serde_json;

mod data_model;
mod store;
use crate::store::{LedgerState};

struct LedgerApp {
    state: LedgerState,
}

impl LedgerApp {
    pub fn new() -> LedgerApp {
        LedgerApp {state: LedgerState::new()}
    }
}

// TODO: implement abci hooks
impl abci::Application for LedgerApp {}

fn main() {
    // Tendermint ABCI port
    let addr = "127.0.0.1:26658".parse().unwrap();

    abci::run(addr, LedgerApp::new());
}
