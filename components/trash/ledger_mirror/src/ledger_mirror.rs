#![deny(warnings)]

use ledger::data_model::{FinalizedTransaction, TxnEffect};
use ledger::store::*;
use ledger_api_service::RestfulApiService;
use log::{error, info};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use std::thread;
use std::time;

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let base_dir = std::env::var_os("LEDGER_DIR").filter(|x| !x.is_empty());
    let base_dir = base_dir.as_ref().map(Path::new);
    env_logger::init();
    let ledger_state = match base_dir {
        None => LedgerState::test_ledger(),
        Some(base_dir) => LedgerState::load_or_init(base_dir).c(d!())?,
    };
    let state_lock = Arc::new(RwLock::new(ledger_state));
    let cloned_lock = Arc::clone(&state_lock);

    let ledger_url = std::env::var_os("LEDGER_URL")
        .filter(|x| !x.is_empty())
        .unwrap_or_else(|| "localhost:8668".into());

    let host = std::env::var_os("SERVER_HOST")
        .filter(|x| !x.is_empty())
        .unwrap_or_else(|| "localhost".into());
    let query_port = std::env::var_os("QUERY_PORT")
        .filter(|x| !x.is_empty())
        .unwrap_or_else(|| "9779".into());

    thread::spawn(move || {
        let query_service = RestfulApiService::create(
            cloned_lock,
            host.to_str().c(d!())?,
            query_port.to_str().c(d!())?,
        )
        .c(d!())?;
        println!("Starting query service");
        match query_service.run() {
            Ok(_) => println!("Successfully ran mirror"),
            Err(_) => println!("Error running mirror"),
        }
    });

    while running.load(Ordering::SeqCst) {
        let poll_time = time::Duration::from_millis(1000);
        thread::sleep(poll_time);
        let latest_block = {
            let ledger = state_lock.read().c(d!())?;
            (*ledger).get_block_count()
        };
        let new_blocks = match attohttpc::get(&format!(
            "http://{}/{}/{}",
            ledger_url.to_str().c(d!())?,
            "blocks_since",
            &latest_block
        )) {
            Err(e) => {
                error!("HTTP Request failed {}", e);
                continue;
            }

            Ok(bs) => match bs.json::<Vec<(usize, Vec<FinalizedTransaction>)>>() {
                Err(e) => {
                    error!("JSON deserialization failed {}", e);
                    continue;
                }
                Ok(bs) => bs,
            },
        };

        let mut ledger = state_lock.write().c(d!())?;
        for (bid, block) in new_blocks {
            info!("Received block {}", bid);
            let mut block_builder = ledger.start_block().c(d!())?;
            for txn in block {
                let txn = txn.txn;
                let eff = TxnEffect::compute_effect(txn).c(d!())?;
                ledger.apply_transaction(&mut block_builder, eff).c(d!())?;
            }
            ledger.finish_block(block_builder).c(d!())?;
        }
    }
}