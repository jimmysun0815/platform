use abci::*;
use baseapp::BaseApp as AccountBaseAPP;
use ledger::address::AddressBinder;
use ledger::store::LedgerState;
use parking_lot::RwLock;
use rand_chacha::ChaChaRng;
use rand_core::SeedableRng;
use ruc::*;
use std::path::Path;
use std::sync::Arc;
use submission_server::SubmissionServer;
use tx_sender::TendermintForward;

pub use tx_sender::forward_txn_with_mode;

pub mod callback;
pub mod tx_sender;

pub struct ABCISubmissionServer {
    pub la: Arc<RwLock<SubmissionServer<ChaChaRng, LedgerState, TendermintForward>>>,
    pub account_base_app: Arc<RwLock<AccountBaseAPP>>,
    pub address_binder: Arc<RwLock<AddressBinder>>,
}

impl ABCISubmissionServer {
    pub fn new(
        base_dir: Option<&Path>,
        tendermint_reply: String,
    ) -> Result<ABCISubmissionServer> {
        let ledger_state = match base_dir {
            None => LedgerState::test_ledger(),
            Some(base_dir) => pnk!(LedgerState::load_or_init(base_dir)),
        };

        let account_base_app = match base_dir {
            None => pnk!(AccountBaseAPP::new(tempfile::tempdir().unwrap().path())),
            Some(base_dir) => pnk!(AccountBaseAPP::new(base_dir)),
        };

        let address_binder = match base_dir {
            None => AddressBinder::test()?,
            Some(base_dir) => {
                pnk!(AddressBinder::new(&base_dir.join("address_binder.db")))
            }
        };

        let prng = rand_chacha::ChaChaRng::from_entropy();
        Ok(ABCISubmissionServer {
            la: Arc::new(RwLock::new(
                SubmissionServer::new_no_auto_commit(
                    prng,
                    Arc::new(RwLock::new(ledger_state)),
                    TendermintForward { tendermint_reply },
                )
                .c(d!())?,
            )),
            account_base_app: Arc::new(RwLock::new(account_base_app)),
            address_binder: Arc::new(RwLock::new(address_binder)),
        })
    }
}

impl abci::Application for ABCISubmissionServer {
    #[inline(always)]
    fn info(&mut self, req: &RequestInfo) -> ResponseInfo {
        callback::info(self, req)
    }

    #[inline(always)]
    fn query(&mut self, req: &RequestQuery) -> ResponseQuery {
        callback::query(self, req)
    }

    #[inline(always)]
    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        callback::check_tx(self, req)
    }

    #[inline(always)]
    fn init_chain(&mut self, req: &RequestInitChain) -> ResponseInitChain {
        callback::init_chain(self, req)
    }

    #[inline(always)]
    fn begin_block(&mut self, req: &RequestBeginBlock) -> ResponseBeginBlock {
        callback::begin_block(self, req)
    }

    #[inline(always)]
    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        callback::deliver_tx(self, req)
    }

    #[inline(always)]
    fn end_block(&mut self, req: &RequestEndBlock) -> ResponseEndBlock {
        callback::end_block(self, req)
    }

    #[inline(always)]
    fn commit(&mut self, req: &RequestCommit) -> ResponseCommit {
        callback::commit(self, req)
    }
}
