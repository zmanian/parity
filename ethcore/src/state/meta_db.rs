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
// along with Parity. If not, see <http://www.gnu.org/licenses/>.

//! Account meta-database.
//!
//! This is a journalled database which stores information on accounts.
//! The instance of `MetaDB` can be pointed towards a specific branch in its journal.
//! Queries for account data will only succeed if they are present in that branch or
//! the backing database.
//!
//! The journal format is two-part. First, for every era we store a list of
//! candidate hashes.
//!
//! For each hash, we store a list of changes in that candidate.

use util::{Address, H256, U256};
use util::kvdb::{Database, DBTransaction};
use rlp::{Decoder, DecoderError, RlpDecodable, RlpEncodable, RlpStream, Stream, Rlp, View};

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;

const PADDING: [u8; 10] = [0; 10];

// generate a key for the given era.
fn journal_key(era: &u64) -> Vec<u8> {
	let mut stream = RlpStream::new_list(3);
	stream.append(&"journal").append(era).append(&&PADDING[..]);
	stream.out()
}

// generate a key for the given id.
fn id_key(id: &H256) -> Vec<u8> {
	let mut stream = RlpStream::new_list(3);
	stream.append(&"journal").append(id).append(&&PADDING[..]);
	stream.out()
}

// deltas in the journal -- these don't contain data for simple rollbacks.
#[derive(Debug, PartialEq)]
enum JournalDelta {
	Destroy,
	Set(AccountMeta),
}

impl RlpEncodable for JournalDelta {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.begin_list(2);
		match *self {
			JournalDelta::Destroy => s.append(&false).append_empty_data(),
			JournalDelta::Set(ref meta) => s.append(&true).append(meta),
		};
	}
}

impl RlpDecodable for JournalDelta {
	fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
		let rlp = decoder.as_rlp();

		Ok(match try!(rlp.val_at::<bool>(0)) {
			true => JournalDelta::Set(try!(rlp.val_at(1))),
			false => JournalDelta::Destroy,
		})
	}
}

// deltas in the branch view -- these contain data making it simple to
// roll back.
#[derive(Debug, PartialEq)]
enum BranchDelta {
	Destroy(AccountMeta),
	Init(AccountMeta),
	Replace(AccountMeta, AccountMeta),
}

impl<'a> From<&'a BranchDelta> for JournalDelta {
	fn from(bd: &'a BranchDelta) -> Self {
		match *bd {
			BranchDelta::Destroy(_) => JournalDelta::Destroy,
			BranchDelta::Init(ref new) | BranchDelta::Replace(ref new, _) => JournalDelta::Set(new.clone()),
		}
	}
}

/// Account meta-information.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AccountMeta {
	/// The size of this account's code.
	pub code_size: usize,
	/// The hash of this account's code.
	pub code_hash: H256,
	/// Storage root for the trie.
	pub storage_root: H256,
	/// Account balance.
	pub balance: U256,
	/// Account nonce.
	pub nonce: U256,
}

impl RlpEncodable for AccountMeta {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.begin_list(5)
			.append(&self.code_size)
			.append(&self.code_hash)
			.append(&self.storage_root)
			.append(&self.balance)
			.append(&self.nonce);
	}
}

impl RlpDecodable for AccountMeta {
	fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
		let rlp = decoder.as_rlp();

		Ok(AccountMeta {
			code_size: try!(rlp.val_at(0)),
			code_hash: try!(rlp.val_at(1)),
			storage_root: try!(rlp.val_at(2)),
			balance: try!(rlp.val_at(3)),
			nonce: try!(rlp.val_at(4)),
		})
	}
}

// The journal used to store meta info.
#[derive(Debug, PartialEq)]
struct Journal {
	// maps block numbers (or more abstractly, eras) to potential canonical meta info.
	entries: BTreeMap<u64, HashMap<H256, JournalEntry>>,
}

impl Journal {
	// read the journal from the database, starting from the last committed
	// era.
	fn read_from(db: &Database, col: Option<u32>, era: u64) -> Result<Self, String> {
		let mut journal = Journal {
			entries: BTreeMap::new(),
		};

		let mut era = era + 1;
		while let Some(hashes) = try!(db.get(col, &journal_key(&era))).map(|x| ::rlp::decode::<Vec<H256>>(&x)) {
			let candidates: Result<HashMap<_, _>, String> = hashes.into_iter().map(|hash| {
				let journal_rlp = try!(db.get(col, &id_key(&hash)))
					.expect(&format!("corrupted database: missing journal data for {}.", hash));

				let rlp = Rlp::new(&journal_rlp);

				Ok((hash, JournalEntry {
					parent: rlp.val_at(0),
					entries: rlp.at(1).iter().map(|e| (e.val_at(0), e.val_at(1))).collect(),
				}))
			}).collect();

			journal.entries.insert(era, try!(candidates));
			era += 1;
		}

		Ok(journal)
	}
}

// Each journal entry stores the parent hash of the block it corresponds to
// and the changes in the meta state it lead to.
#[derive(Debug, PartialEq)]
struct JournalEntry {
	parent: H256,
	entries: Vec<(Address, JournalDelta)>,
}

/// Reorganization is impossible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReorgImpossible;

/// The account meta-database. See the module docs for more details.
/// It can't be queried without a `MetaBranch` which allows for accurate
/// queries along the current branch.
///
/// This has a short journal period, and is only really usable while syncing.
/// When replaying old transactions, it can't be used safely.
pub struct MetaDB {
	col: Option<u32>,
	db: Arc<Database>,
	journal: Journal,
	last_committed: (H256, u64), // last committed era.
	branch: MetaBranch, // current branch.
}

impl MetaDB {
	/// Create a new `MetaDB` from a database and column. This will also load the journal.
	///
	/// After creation, check the last committed era to see if the genesis state
	/// is in. If not, it should be inserted, journalled, and marked canonical.
	pub fn new(db: Arc<Database>, col: Option<u32>, genesis_hash: &H256) -> Result<Self, String> {
		let last_committed: (H256, u64) = try!(db.get(col, b"latest")).map(|raw| {
			let rlp = Rlp::new(&raw);

			(rlp.val_at(0), rlp.val_at(1))
		}).unwrap_or_else(|| (genesis_hash.clone(), 0));

		let journal = try!(Journal::read_from(&*db, col, last_committed.1));

		Ok(MetaDB {
			col: col,
			db: db,
			journal: journal,
			last_committed: last_committed.clone(),
			branch: MetaBranch {
				ancestors: VecDeque::new(),
				current_changes: Vec::new(),
				overlay: HashMap::new(),
				recent: HashMap::new(),
			}
		})
	}

	/// Journal all pending changes under the given era and id. Also updates
	/// the branch view to point at this era.
	pub fn journal_under(&mut self, batch: &mut DBTransaction, now: u64, id: H256, parent_id: H256) {
		trace!(target: "meta_db", "journalling ({}, {})", now, id);

		// convert meta branch pending changes to journal entry.
		let pending: Vec<(Address, JournalDelta)> = self.branch.current_changes
			.iter()
			.map(|&(ref addr, ref delta)| (addr.clone(), delta.into()))
			.collect();

		// write out the new journal entry.
		{
			let key = id_key(&id);
			let mut stream = RlpStream::new_list(2);
			stream.append(&parent_id);
			stream.begin_list(pending.len());

			for &(ref addr, ref delta) in pending.iter() {
				stream.begin_list(2);
				stream.append(addr).append(delta);
			}

			batch.put_vec(self.col, &key, stream.out());
		}

		self.branch.accrue(id);
		let candidates: Vec<H256> = {
			let entries = self.journal.entries.entry(now).or_insert_with(HashMap::new);
			entries.insert(id, JournalEntry {
				parent: parent_id,
				entries: pending,
			});

			entries.keys().cloned().collect()
		};


		// write out the new ids key.
		{
			let key = journal_key(&now);
			batch.put_vec(self.col, &key, ::rlp::encode(&candidates).to_vec());
		}
	}

	/// Mark an era as canonical. May invalidate the current branch view.
	///
	/// This immediately sets the last committed hash, leading to a potential
	/// race condition if the meta DB is when .
	/// As such, it's not suitable to be used outside of the main sync.
	pub fn mark_canonical(&mut self, batch: &mut DBTransaction, end_era: u64, canon_id: H256) {
		trace!(target: "meta_db", "mark_canonical: ({}, {})", end_era, canon_id);

		let entries = match self.journal.entries.remove(&end_era) {
			Some(entries) => entries,
			None => {
				warn!("No meta DB journal for era={}", end_era);
				return;
			}
		};

		// TODO: delete old branches building off of invalidated candidates.
		for (id, entry) in entries {
			let key = id_key(&id);
			batch.delete(self.col, &key[..]);

			if id == canon_id {
				for (address, delta) in entry.entries {
					match delta {
						JournalDelta::Destroy => batch.delete(self.col, &*address),
						JournalDelta::Set(meta) =>
							batch.put_vec(self.col, &*address, ::rlp::encode(&meta).to_vec()),
					}
				}
			}
		}

		// remove the list of hashes.
		let key = journal_key(&end_era);
		batch.delete(self.col, &key[..]);

		self.last_committed = (canon_id, end_era);

		// prune the branch view and reset it if it's based off a non-canonical
		// block.
		if !self.branch.remove_ancient(&canon_id) {
			self.clear_branch();
		}

		{
			let mut stream = RlpStream::new_list(2);
			stream.append(&self.last_committed.0).append(&self.last_committed.1);
			batch.put_vec(self.col, b"latest", stream.out())
		}
	}

	/// Query the state of an account. A return value
	/// of `None` means that the account does not exist on this branch.
	pub fn get(&self, address: &Address) -> Option<AccountMeta> {
		self.branch.overlay.get(address).map(|o| o.clone()).unwrap_or_else(|| {
			match self.db.get(self.col, &*address) {
				Ok(maybe) => maybe.map(|x| ::rlp::decode(&x)),
				Err(e) => {
					warn!("Low-level database error: {}", e);
					None
				}
			}
		})
	}

	/// Set the given account's details on this address.
	/// This will completely overwrite any other entry.
	pub fn set(&mut self, address: Address, meta: AccountMeta) {
		match self.get(&address) {
			Some(prev) => {
				self.branch.overlay.insert(address.clone(), Some(meta.clone()));
				self.branch.current_changes.push((address, BranchDelta::Replace(meta, prev)));
			}
			None => {
				self.branch.overlay.insert(address.clone(), Some(meta.clone()));
				self.branch.current_changes.push((address, BranchDelta::Init(meta)));
			}
		}
	}

	/// Destroy the account details here.
	pub fn remove(&mut self, address: Address) {
		// `None` shouldn't be strictly possible, but we actually re-remove all
		// accounts in the state cache which were just nonexistant at the time of
		// query.
		if let Some(prev) = self.get(&address) {
			self.branch.overlay.insert(address.clone(), None);
			self.branch.current_changes.push((address, BranchDelta::Destroy(prev)));
		}
	}

	/// Set the head to the requested branch.
	/// The block must be in the journal already.
	///
	/// Will fail if the common ancestor and both branches aren't in the journal.
	/// This shouldn't be possible for anything within the history period.
	///
	/// Note that this will point to the meta state at the point immediately
	/// after the given id.
	///
	/// On failure, branch state is undefined and must be set to a possible
	/// branch before continued use.
	pub fn branch_to(&mut self, hash: H256, new_era: u64) -> Result<(), ReorgImpossible> {
		trace!(target: "meta_db", "branch to ({}, {})", new_era, hash);

		// as a macro since closures borrow.
		macro_rules! journal_entry {
			($era: expr, $id: expr) => {{
				trace!(target: "meta_db", "fetching journal entry: ({}, {})", $era, $id);

				self.journal.entries.get($era)
					.and_then(|entries| entries.get($id)).ok_or(ReorgImpossible)
			}}
		}

		// first things first, clear any uncommitted changes on the branch.
		self.branch.clear_current();

		if new_era == self.last_committed.1 {
			self.clear_branch();

			return if hash != self.last_committed.0 {
				Err(ReorgImpossible)
			} else {
				Ok(())
			}
		}

		// check for equivalent branch.
		{
			let branch_head = self.branch.latest_id()
				.unwrap_or(&self.last_committed.0);

			if new_era == self.branch_era() && branch_head == &hash { return Ok(()) }
		}

		let mut to_era = new_era;
		let mut to_branch = vec![];
		let mut ancestor_hash = hash.clone();

		trace!(target: "meta_db", "reorg necessary: branch_era={}, new_era={}", self.branch_era(), new_era);

		// reset to same level by rolling back the branch
		while self.branch_era() > to_era {
			trace!(target: "meta_db", "rolling back branch once.");

			// protected by check above.
			self.branch.rollback().expect("branch known to have enough journalled ancestors; qed");
		}

		while to_era > self.branch_era() {
			let entry = try!(journal_entry!(&to_era, &ancestor_hash));

			to_branch.push(ancestor_hash);
			ancestor_hash = entry.parent;
			to_era -= 1;
		}

		// rewind the branch until we find a common ancestor
		while try!(self.branch.latest_id().ok_or(ReorgImpossible)) != &ancestor_hash {
			trace!(target: "meta_db", "rolling back branch");

			try!(self.branch.rollback().ok_or(ReorgImpossible));

			let entry = try!(journal_entry!(&to_era, &ancestor_hash));

			to_branch.push(ancestor_hash);
			ancestor_hash = entry.parent;
			to_era -= 1;
		}

		// and then walk forward, accruing all of the fork branch's changes into
		// the branch.
		for (era, id) in (to_era..new_era).zip(to_branch.into_iter().rev()) {
			let entry = journal_entry!(&era, &id).expect("this entry fetched previously; qed");

			self.branch.accrue_journal(id, &entry.entries, &*self.db, self.col);
		}

		assert_eq!(self.branch_era(), new_era);
		assert_eq!(self.branch.latest_id(), Some(&hash));

		Ok(())
	}

	/// Whether the meta database is empty.
	pub fn is_empty(&self) -> bool {
		self.last_committed.1 > 0 || self.journal.entries.is_empty()
	}

	// set the branch to completely empty.
	fn clear_branch(&mut self) {
		self.branch = MetaBranch {
			ancestors: VecDeque::new(),
			current_changes: Vec::new(),
			overlay: HashMap::new(),
			recent: HashMap::new(),
		};
	}

	// the branch's era.
	fn branch_era(&self) -> u64 {
		self.last_committed.1 + self.branch.len()
	}
}

// A reorg-friendly view over a branch based on the `MetaDB`.
#[derive(Debug, PartialEq)]
struct MetaBranch {
	ancestors: VecDeque<(H256, Vec<(Address, BranchDelta)>)>,
	current_changes: Vec<(Address, BranchDelta)>,

	// current state of account meta, accruing from the database's last
	// to the current changes. `None` means killed, missing means no change from db,
	// present means known value.
	overlay: HashMap<Address, Option<AccountMeta>>,

	// recently touched addresses -- maps addresses to refcount.
	// when we pop an ancestor. current changes aren't counted
	// until accrued.
	recent: HashMap<Address, u32>,
}

impl MetaBranch {
	// The length of this branch.
	fn len(&self) -> u64 { self.ancestors.len() as u64 }

	// latest tracked id.
	fn latest_id(&self) -> Option<&H256> {
		self.ancestors.back().map(|&(ref hash, _)| hash)
	}

	// clear current changes.
	fn clear_current(&mut self) {
		self.current_changes.clear()
	}

	// pop an ancestor and roll back its changes. Returns the hash
	// of the ancestor just popped, or none if there isn't one.
	fn rollback(&mut self) -> Option<H256> {
		match self.ancestors.pop_back() {
			Some((hash, delta)) => {
				// process changes in reverse for proper backtracking.
				for &(ref address, ref delta) in delta.iter().rev() {
					match *delta {
						BranchDelta::Destroy(ref prev) => self.overlay.insert(address.clone(), Some(prev.clone())),
						BranchDelta::Init(_) => self.overlay.remove(address),
						BranchDelta::Replace(ref prev, _) => self.overlay.insert(address.clone(), Some(prev.clone())),
					};
				}

				// prune out anything that only this touched after the rollback,
				// in case something from the backing database was restored.
				self.prune_recent(delta.into_iter().map(|(addr, _)| addr));

				Some(hash)
			}
			None => None,
		}
	}

	// Accrue current changes under the given hash, incrementing the latest era.
	fn accrue(&mut self, hash: H256) {
		let current_changes = ::std::mem::replace(&mut self.current_changes, Vec::new());

		// mark these items as recently changed.
		for addr in current_changes.iter().map(|&(ref addr, _)| addr).cloned() {
			*self.recent.entry(addr).or_insert(0) += 1;
		}

		self.ancestors.push_back((hash, current_changes));
	}

	// Accrue deltas from the journal.
	// The hash deltas here must immediately follow the block this branch tracks.
	// This is a relatively expensive operation, but should only be triggered on reorganizations.
	fn accrue_journal(&mut self, hash: H256, j_deltas: &[(Address, JournalDelta)], db: &Database, col: Option<u32>) {
		let mut deltas = Vec::with_capacity(j_deltas.len());

		for &(ref addr, ref j_delta) in j_deltas {
			// update the recent hashmap to denote this item.
			*self.recent.entry(addr.clone()).or_insert(0) += 1;

			let prev: Option<AccountMeta> = self.overlay.get(addr).map(|o| o.clone()).unwrap_or_else(|| {
				match db.get(col, &*addr) {
					Ok(maybe_prev) => maybe_prev.map(|x| ::rlp::decode(&x)),
					Err(e) => {
						warn!("Low-level database error: {}", e);
						None
					}
				}
			});

			let delta = match *j_delta {
				JournalDelta::Destroy => {
					let prev = prev.expect("cannot destroy without account existing; qed");
					self.overlay.insert(addr.clone(), None);

					BranchDelta::Destroy(prev)
				}
				JournalDelta::Set(ref meta) => {
					self.overlay.insert(addr.clone(), Some(meta.clone()));

					match prev {
						Some(prev) => BranchDelta::Replace(meta.clone(), prev),
						None => BranchDelta::Init(meta.clone()),
					}
				}
			};

			deltas.push((addr.clone(), delta));
		}

		self.ancestors.push_back((hash, deltas));
	}

	// decrement the refcount in `recent` for any address in the given
	// iterable.
	fn prune_recent<I>(&mut self, iter: I) where I: IntoIterator<Item=Address> {
		use std::collections::hash_map::Entry;

		for addr in iter {
			match self.recent.entry(addr) {
				Entry::Occupied(mut x) => {
					*x.get_mut() -= 1;
					if *x.get() == 0 {
						x.remove();
					}
				}
				_ => {}
			}
		}
	}

	// get rid of the most ancient ancestor, and remove any stale entries from the overlay.
	// if the ancient ancestor isn't equal to the canon id, returns false, true otherwise.
	// this signals that the branch needs to be wiped.
	fn remove_ancient(&mut self, canon_id: &H256) -> bool {
		let delta = match self.ancestors.pop_front() {
			Some((hash, delta)) => if &hash == canon_id {
				trace!(target: "meta_db", "removed ancient ancestor {} from branch.", hash);
				delta
			} else {
				return false
			},
			_ => { return false },
		};

		self.prune_recent(delta.into_iter().map(|(addr, _)| addr));
		true
	}
}

#[cfg(test)]
mod tests {
	use super::{AccountMeta, MetaDB};
	use devtools::RandomTempPath;

	use util::{U256, H256};
	use util::kvdb::Database;

	use std::sync::Arc;

	#[test]
	fn loads_journal() {
		let path = RandomTempPath::create_dir();
		let db = Arc::new(Database::open_default(&*path.as_path().to_string_lossy()).unwrap());
		let mut meta_db = MetaDB::new(db.clone(), None, &Default::default()).unwrap();

		for i in 0..10 {
			let this = U256::from(i + 1);
			let parent = U256::from(i);

			let mut batch = db.transaction();
			meta_db.journal_under(&mut batch, i + 1, this.into(), parent.into());
			db.write(batch).unwrap();
		}

		let journal = meta_db.journal;

		let meta_db = MetaDB::new(db.clone(), None, &Default::default()).unwrap();

		assert_eq!(journal, meta_db.journal);
	}

	#[test]
	fn mark_canonical_keeps_branch() {
		let path = RandomTempPath::create_dir();
		let db = Arc::new(Database::open_default(&*path.as_path().to_string_lossy()).unwrap());
		let mut meta_db = MetaDB::new(db.clone(), None, &Default::default()).unwrap();

		for i in 0..10 {
			let this = U256::from(i + 1);
			let parent = U256::from(i);

			let mut batch = db.transaction();
			meta_db.journal_under(&mut batch, i + 1, this.into(), parent.into());
			db.write(batch).unwrap();
		}

		assert_eq!(meta_db.branch_era(), 10);

		let mut batch = db.transaction();
		meta_db.mark_canonical(&mut batch, 1, U256::from(1).into());
		db.write(batch).unwrap();

		assert_eq!(meta_db.branch_era(), 10);
	}
}