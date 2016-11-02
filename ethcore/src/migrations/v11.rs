// Copyright 2015, 2016 Ethcore (UK) Ltd.
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

//! MetaDB upgrade.

use rlp::{RlpStream, Stream};

use util::kvdb::Database;
use util::journaldb::{self, Algorithm};
use util::migration::{Batch, Config, Error, Migration, Progress};
use util::trie::{Trie, TrieDB};
use util::hash::{FixedHash, H256};

use state::Account;
use views::HeaderView;

use std::sync::Arc;

// constants reproduced here for backwards compatibility.
pub const COL_STATE: Option<u32> = Some(0);
pub const COL_HEADERS: Option<u32> = Some(1);
pub const COL_EXTRA: Option<u32> = Some(3);
pub const COL_META: Option<u32> = Some(6);
pub const PRE_COLUMNS: Option<u32> = Some(6);
pub const POST_COLUMNS: Option<u32> = Some(7);

/// The 10 -> 11 migration: fills the MetaDB with account information.
pub struct ToV11(Progress, Algorithm);

impl ToV11 {
	/// Create a new ToV11 migration with a progress indicator and
	/// pruning algorithm.
	pub fn new(pruning: Algorithm) -> Self {
		ToV11(Progress::default(), pruning)
	}
}

impl Migration for ToV11 {
	fn pre_columns(&self) -> Option<u32> { PRE_COLUMNS }
	fn columns(&self) -> Option<u32> { POST_COLUMNS }
	fn version(&self) -> u32 { 11 }

	fn migrate(&mut self, source: Arc<Database>, config: &Config, dest: &mut Database, col: Option<u32>) -> Result<(), Error> {
		macro_rules! try_fmt {
			($e: expr) => {
				try!(($e).map_err(|e| format!("{}", e)))
			}
		}

		let mut batch = Batch::new(config, col);

		// first do a simple copy.
		for (key, value) in source.iter(col) {
			self.0.tick();
			try!(batch.insert(key.to_vec(), value.to_vec(), dest));
		}

		try!(batch.commit(dest));

		// next portion relevant for the state column only.
		if col != COL_STATE { return Ok(()) }

		let mut batch = Batch::new(config, COL_META);

		// load the best block's header.
		let best = match try!(source.get(COL_EXTRA, b"best")) {
			Some(best) => best,
			None => return Ok(())
		};

		let best_header = match try!(source.get(COL_HEADERS, &best)) {
			Some(header) => header,
			None => return Err(Error::Custom("Database corruption encountered: no best block header".into())),
		};

		let best_header = HeaderView::new(&best_header);
		let state_root = best_header.state_root();

		let journaldb = journaldb::new(source.clone(), self.1, COL_STATE);

		// iterate the trie, writing each item directly into the meta_db column.
		let trie = try_fmt!(TrieDB::new(journaldb.as_hashdb(), &state_root));
		for item in try_fmt!(trie.iter()) {
			let (addr_hash, acc_rlp) = try_fmt!(item);

			let meta = {
				let mut account = Account::from_rlp(&acc_rlp);
				let account_db = ::account_db::AccountDB::from_hash(journaldb.as_hashdb(), H256::from_slice(&addr_hash));
				account.cache_code_size(&account_db);

				account.account_meta(None)
			};

			self.0.tick();
			try!(batch.insert(addr_hash, ::rlp::encode(&meta).to_vec(), dest));
		}

		println!("Setting meta DB state to ({}, {})", best_header.number(), best_header.hash());

		let mut base_stream = RlpStream::new_list(2);
		base_stream.append(&best_header.number()).append(&best_header.hash());

		try!(batch.insert(b"base".to_vec(), base_stream.out(), dest));

		batch.commit(dest)
	}
}