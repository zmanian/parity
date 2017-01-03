// Copyright 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

//! rpc integration tests.

use devtools::RandomTempPath;
use ethcore::account_provider::AccountProvider;
use ethcore::block::Block;

use ethcore::client::{BlockChainClient, Client, ClientConfig};
use ethcore::ethereum;
use ethcore::ids::BlockId;
use ethcore::miner::{MinerOptions, Banning, GasPricer, MinerService, ExternalMiner, Miner, PendingSet, PrioritizationStrategy, GasLimit};
use ethcore::spec::{Genesis, Spec};
use ethcore::views::BlockView;
use ethjson::blockchain::BlockChain;
use io::IoChannel;
use jsonrpc_core::{IoHandler, GenericIoHandler};
use std::sync::Arc;
use std::time::Duration;
use util::{U256, H256, Uint, Address};
use util::Hashable;

use v1::impls::{EthClient, SigningUnsafeClient};
use v1::tests::helpers::{TestSnapshotService, TestSyncProvider, Config};
use v1::traits::eth::Eth;
use v1::traits::eth_signing::EthSigning;
use v1::types::U256 as NU256;

fn account_provider() -> Arc<AccountProvider> {
	Arc::new(AccountProvider::transient_provider())
}

fn sync_provider() -> Arc<TestSyncProvider> {
	Arc::new(TestSyncProvider::new(Config { network_id: 3, num_peers: 120 }))
}

fn miner_service(spec: &Spec, accounts: Arc<AccountProvider>) -> Arc<Miner> {
	Miner::new(MinerOptions {
		           new_work_notify: vec![],
		           force_sealing: true,
		           reseal_on_external_tx: true,
		           reseal_on_own_tx: true,
		           tx_queue_size: 1024,
		           tx_gas_limit: !U256::zero(),
		           tx_queue_strategy: PrioritizationStrategy::GasPriceOnly,
		           tx_queue_gas_limit: GasLimit::None,
		           tx_queue_banning: Banning::Disabled,
		           pending_set: PendingSet::SealingOrElseQueue,
		           reseal_min_period: Duration::from_secs(0),
		           work_queue_size: 50,
		           enable_resubmission: true,
	           },
	           GasPricer::new_fixed(20_000_000_000u64.into()),
	           &spec,
	           Some(accounts))
}

fn snapshot_service() -> Arc<TestSnapshotService> {
	Arc::new(TestSnapshotService::new())
}

fn make_spec(chain: &BlockChain) -> Spec {
	let genesis = Genesis::from(chain.genesis());
	let mut spec = ethereum::new_frontier_test();
	let state = chain.pre_state.clone().into();
	spec.set_genesis_state(state);
	spec.overwrite_genesis_params(genesis);
	assert!(spec.is_state_root_valid());
	spec
}

struct EthTester {
	client: Arc<Client>,
	_miner: Arc<MinerService>,
	_snapshot: Arc<TestSnapshotService>,
	accounts: Arc<AccountProvider>,
	handler: IoHandler,
}

impl EthTester {
	fn from_chain(chain: &BlockChain) -> Self {
		let tester = Self::from_spec(make_spec(chain));

		for b in &chain.blocks_rlp() {
			if Block::is_good(&b) {
				let _ = tester.client.import_block(b.clone());
				tester.client.flush_queue();
				tester.client.import_verified_blocks();
			}
		}

		tester.client.flush_queue();

		assert!(tester.client.chain_info().best_block_hash == chain.best_block.clone().into());
		tester
	}

	fn from_spec(spec: Spec) -> Self {
		let dir = RandomTempPath::new();
		let account_provider = account_provider();
		spec.engine.register_account_provider(account_provider.clone());
		let miner_service = miner_service(&spec, account_provider.clone());
		let snapshot_service = snapshot_service();

		let db_config = ::util::kvdb::DatabaseConfig::with_columns(::ethcore::db::NUM_COLUMNS);
		let client = Client::new(ClientConfig::default(), &spec, dir.as_path(), miner_service.clone(), IoChannel::disconnected(), &db_config).unwrap();
		let sync_provider = sync_provider();
		let external_miner = Arc::new(ExternalMiner::default());

		let eth_client = EthClient::new(&client, &snapshot_service, &sync_provider, &account_provider, &miner_service, &external_miner, Default::default());
		let eth_sign = SigningUnsafeClient::new(&client, &account_provider, &miner_service);

		let handler = IoHandler::new();
		handler.add_delegate(eth_client.to_delegate());
		handler.add_delegate(eth_sign.to_delegate());

		EthTester {
			_miner: miner_service,
			_snapshot: snapshot_service,
			client: client,
			accounts: account_provider,
			handler: handler,
		}
	}
}

#[test]
fn harness_works() {
	let chain: BlockChain = extract_chain!("BlockchainTests/bcUncleTest");
	let _ = EthTester::from_chain(&chain);
}

#[test]
fn eth_get_balance() {
	let chain = extract_chain!("BlockchainTests/bcWalletTest", "wallet2outOf3txs");
	let tester = EthTester::from_chain(&chain);
	// final account state
	let req_latest = r#"{
		"jsonrpc": "2.0",
		"method": "eth_getBalance",
		"params": ["0xaaaf5374fce5edbc8e2a8697c15331677e6ebaaa", "latest"],
		"id": 1
	}"#;
	let res_latest = r#"{"jsonrpc":"2.0","result":"0x9","id":1}"#.to_owned();
	assert_eq!(tester.handler.handle_request_sync(req_latest).unwrap(), res_latest);

	// non-existant account
	let req_new_acc = r#"{
		"jsonrpc": "2.0",
		"method": "eth_getBalance",
		"params": ["0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
		"id": 3
	}"#;

	let res_new_acc = r#"{"jsonrpc":"2.0","result":"0x0","id":3}"#.to_owned();
	assert_eq!(tester.handler.handle_request_sync(req_new_acc).unwrap(), res_new_acc);
}

#[test]
fn eth_block_number() {
	let chain = extract_chain!("BlockchainTests/bcRPC_API_Test");
	let tester = EthTester::from_chain(&chain);
	let req_number = r#"{
		"jsonrpc": "2.0",
		"method": "eth_blockNumber",
		"params": [],
		"id": 1
	}"#;

	let res_number = r#"{"jsonrpc":"2.0","result":"0x20","id":1}"#.to_owned();
	assert_eq!(tester.handler.handle_request_sync(req_number).unwrap(), res_number);
}

// a frontier-like test with an expanded gas limit and balance on known account.
const TRANSACTION_COUNT_SPEC: &'static [u8] = br#"{
	"name": "Frontier (Test)",
	"engine": {
		"Ethash": {
			"params": {
				"gasLimitBoundDivisor": "0x0400",
				"minimumDifficulty": "0x020000",
				"difficultyBoundDivisor": "0x0800",
				"durationLimit": "0x0d",
				"blockReward": "0x4563918244F40000",
				"registrar" : "0xc6d9d2cd449a754c494264e1809c50e34d64562b",
				"homesteadTransition": "0xffffffffffffffff",
				"daoHardforkTransition": "0xffffffffffffffff",
				"daoHardforkBeneficiary": "0x0000000000000000000000000000000000000000",
				"daoHardforkAccounts": []
			}
		}
	},
	"params": {
		"accountStartNonce": "0x00",
		"maximumExtraDataSize": "0x20",
		"minGasLimit": "0x50000",
		"networkID" : "0x1"
	},
	"genesis": {
		"seal": {
			"ethereum": {
				"nonce": "0x0000000000000042",
				"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
			}
		},
		"difficulty": "0x400000000",
		"author": "0x0000000000000000000000000000000000000000",
		"timestamp": "0x00",
		"parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
		"extraData": "0x11bbe8db4e347b4e8c937c1c8370e4b5ed33adb3db69cbdb7a38e1e50b1b82fa",
		"gasLimit": "0x50000"
	},
	"accounts": {
		"0000000000000000000000000000000000000001": { "builtin": { "name": "ecrecover", "pricing": { "linear": { "base": 3000, "word": 0 } } } },
		"0000000000000000000000000000000000000002": { "builtin": { "name": "sha256", "pricing": { "linear": { "base": 60, "word": 12 } } } },
		"0000000000000000000000000000000000000003": { "builtin": { "name": "ripemd160", "pricing": { "linear": { "base": 600, "word": 120 } } } },
		"0000000000000000000000000000000000000004": { "builtin": { "name": "identity", "pricing": { "linear": { "base": 15, "word": 3 } } } },
		"0000000000000000000000000000000000000005": { "builtin": { "name": "blake3b", "pricing": { "linear": { "base": 60, "word": 12 } } } },
		"faa34835af5c2ea724333018a515fbb7d5bc0b33": { "balance": "10000000000000", "nonce": "0" }
	}
}
"#;

const POSITIVE_NONCE_SPEC: &'static [u8] = br#"{
	"name": "Frontier (Test)",
	"engine": {
		"Ethash": {
			"params": {
				"gasLimitBoundDivisor": "0x0400",
				"minimumDifficulty": "0x020000",
				"difficultyBoundDivisor": "0x0800",
				"durationLimit": "0x0d",
				"blockReward": "0x4563918244F40000",
				"registrar" : "0xc6d9d2cd449a754c494264e1809c50e34d64562b",
				"homesteadTransition": "0xffffffffffffffff",
				"daoHardforkTransition": "0xffffffffffffffff",
				"daoHardforkBeneficiary": "0x0000000000000000000000000000000000000000",
				"daoHardforkAccounts": []
			}
		}
	},
	"params": {
		"accountStartNonce": "0x0100",
		"maximumExtraDataSize": "0x20",
		"minGasLimit": "0x50000",
		"networkID" : "0x1"
	},
	"genesis": {
		"seal": {
			"ethereum": {
				"nonce": "0x0000000000000042",
				"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
			}
		},
		"difficulty": "0x400000000",
		"author": "0x0000000000000000000000000000000000000000",
		"timestamp": "0x00",
		"parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
		"extraData": "0x11bbe8db4e347b4e8c937c1c8370e4b5ed33adb3db69cbdb7a38e1e50b1b82fa",
		"gasLimit": "0x50000"
	},
	"accounts": {
		"0000000000000000000000000000000000000001": { "builtin": { "name": "ecrecover", "pricing": { "linear": { "base": 3000, "word": 0 } } } },
		"0000000000000000000000000000000000000002": { "builtin": { "name": "sha256", "pricing": { "linear": { "base": 60, "word": 12 } } } },
		"0000000000000000000000000000000000000003": { "builtin": { "name": "ripemd160", "pricing": { "linear": { "base": 600, "word": 120 } } } },
		"0000000000000000000000000000000000000004": { "builtin": { "name": "identity", "pricing": { "linear": { "base": 15, "word": 3 } } } },
		"faa34835af5c2ea724333018a515fbb7d5bc0b33": { "balance": "10000000000000", "nonce": "0" }
	}
}
"#;

#[test]
fn eth_transaction_count() {
	let secret = "8a283037bb19c4fed7b1c569e40c7dcff366165eb869110a1b11532963eb9cb2".into();
	let tester = EthTester::from_spec(Spec::load(TRANSACTION_COUNT_SPEC).expect("invalid chain spec"));
	let address = tester.accounts.insert_account(secret, "").unwrap();
	tester.accounts.unlock_account_permanently(address, "".into()).unwrap();

	let req_before = r#"{
		"jsonrpc": "2.0",
		"method": "eth_getTransactionCount",
		"params": [""#
	.to_owned() + format!("0x{:?}", address).as_ref() +
	                 r#"", "latest"],
		"id": 15
	}"#;

	let res_before = r#"{"jsonrpc":"2.0","result":"0x0","id":15}"#;

	assert_eq!(tester.handler.handle_request_sync(&req_before).unwrap(), res_before);

	let req_send_trans = r#"{
		"jsonrpc": "2.0",
		"method": "eth_sendTransaction",
		"params": [{
			"from": ""#
	.to_owned() + format!("0x{:?}", address).as_ref() +
	                     r#"",
			"to": "0xd46e8dd67c5d32be8058bb8eb970870f07244567",
			"gas": "0x30000",
			"gasPrice": "0x1",
			"value": "0x9184e72a"
		}],
		"id": 16
	}"#;

	// dispatch the transaction.
	tester.handler.handle_request_sync(&req_send_trans).unwrap();

	// we have submitted the transaction -- but this shouldn't be reflected in a "latest" query.
	let req_after_latest = r#"{
		"jsonrpc": "2.0",
		"method": "eth_getTransactionCount",
		"params": [""#
	.to_owned() + format!("0x{:?}", address).as_ref() +
	                       r#"", "latest"],
		"id": 17
	}"#;

	let res_after_latest = r#"{"jsonrpc":"2.0","result":"0x0","id":17}"#;

	assert_eq!(&tester.handler.handle_request_sync(&req_after_latest).unwrap(), res_after_latest);

	// the pending transactions should have been updated.
	let req_after_pending = r#"{
		"jsonrpc": "2.0",
		"method": "eth_getTransactionCount",
		"params": [""#
	.to_owned() + format!("0x{:?}", address).as_ref() +
	                        r#"", "pending"],
		"id": 18
	}"#;

	let res_after_pending = r#"{"jsonrpc":"2.0","result":"0x1","id":18}"#;

	assert_eq!(&tester.handler.handle_request_sync(&req_after_pending).unwrap(), res_after_pending);
}

fn verify_transaction_counts(name: String, chain: BlockChain) {
	struct PanicHandler(String);
	impl Drop for PanicHandler {
		fn drop(&mut self) {
			if ::std::thread::panicking() {
				println!("Test failed: {}", self.0);
			}
		}
	}

	let _panic = PanicHandler(name);

	fn by_hash(hash: H256, count: usize, id: &mut usize) -> (String, String) {
		let req = r#"{
			"jsonrpc": "2.0",
			"method": "eth_getBlockTransactionCountByHash",
			"params": [
				""#
		.to_owned() + format!("0x{:?}", hash).as_ref() +
		          r#""
			],
			"id": "# + format!("{}", *id).as_ref() +
		          r#"
		}"#;

		let res = r#"{"jsonrpc":"2.0","result":""#.to_owned() + format!("0x{:x}", count).as_ref() + r#"","id":"# + format!("{}", *id).as_ref() + r#"}"#;
		*id += 1;
		(req, res)
	}

	fn by_number(num: u64, count: usize, id: &mut usize) -> (String, String) {
		let req = r#"{
			"jsonrpc": "2.0",
			"method": "eth_getBlockTransactionCountByNumber",
			"params": [
				"#
		.to_owned() + &::serde_json::to_string(&NU256::from(num)).unwrap() +
		          r#"
			],
			"id": "# + format!("{}", *id).as_ref() +
		          r#"
		}"#;

		let res = r#"{"jsonrpc":"2.0","result":""#.to_owned() + format!("0x{:x}", count).as_ref() + r#"","id":"# + format!("{}", *id).as_ref() + r#"}"#;
		*id += 1;
		(req, res)
	}

	let tester = EthTester::from_chain(&chain);

	let mut id = 1;
	for b in chain.blocks_rlp().iter().filter(|b| Block::is_good(b)).map(|b| BlockView::new(b)) {
		let count = b.transactions_count();

		let hash = b.sha3();
		let number = b.header_view().number();

		let (req, res) = by_hash(hash, count, &mut id);
		assert_eq!(tester.handler.handle_request_sync(&req), Some(res));

		// uncles can share block numbers, so skip them.
		if tester.client.block_hash(BlockId::Number(number)) == Some(hash) {
			let (req, res) = by_number(number, count, &mut id);
			assert_eq!(tester.handler.handle_request_sync(&req), Some(res));
		}
	}
}

#[test]
fn starting_nonce_test() {
	let tester = EthTester::from_spec(Spec::load(POSITIVE_NONCE_SPEC).expect("invalid chain spec"));
	let address = Address::from(10);

	let sample = tester.handler
	.handle_request_sync(&(r#"
		{
			"jsonrpc": "2.0",
			"method": "eth_getTransactionCount",
			"params": [""#
	.to_owned() + format!("0x{:?}", address).as_ref() +
	                       r#"", "latest"],
			"id": 15
		}
		"#))
	.unwrap();

	assert_eq!(r#"{"jsonrpc":"2.0","result":"0x100","id":15}"#, &sample);
}

register_test!(eth_transaction_count_1, verify_transaction_counts, "BlockchainTests/bcWalletTest");
register_test!(eth_transaction_count_2, verify_transaction_counts, "BlockchainTests/bcTotalDifficultyTest");
register_test!(eth_transaction_count_3, verify_transaction_counts, "BlockchainTests/bcGasPricerTest");
