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
//! It is implemented using a configurable journal (following a similar API to JournalDB)
//! which builds off of an on-disk flat representation of the state for the last committed block.
//!
//! Any query about an account can be definitively answered for any block in the journal
//! or the canonical base.
//!
//! The journal format is two-part. First, for every era we store a list of
//! candidate hashes.
//!
//! For each hash, we store a list of changes in that candidate.

use util::{HeapSizeOf, H256, U256, RwLock};
use util::kvdb::{Database, DBTransaction};
use rlp::{Decoder, DecoderError, RlpDecodable, RlpEncodable, RlpStream, Stream, Rlp, View};

use std::collections::{BTreeMap, HashMap, BTreeSet};
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

/// Errors which can occur in the operation of the meta db.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
	/// A database error.
	Database(String),
	/// No journal entry found for the specified era, id.
	MissingJournalEntry(u64, H256),
	/// Request made for pruned state.
	StatePruned(u64, H256),
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

known_heap_size!(0, AccountMeta);

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

// Each journal entry stores the parent hash of the block it corresponds to
// and the changes in the meta state it lead to.
#[derive(Debug, PartialEq)]
struct JournalEntry {
	parent: H256,
	// every entry which was set for this era.
	entries: HashMap<H256, Option<AccountMeta>>,
}

impl HeapSizeOf for JournalEntry {
	fn heap_size_of_children(&self) -> usize {
		self.entries.heap_size_of_children()
	}
}

impl RlpEncodable for JournalEntry {
	fn rlp_append(&self, s: &mut RlpStream) {
		s.begin_list(2);
		s.append(&self.parent);

		s.begin_list(self.entries.len());
		for (acc, delta) in self.entries.iter() {
			s.begin_list(2).append(acc);
			s.begin_list(2);

			match *delta {
				Some(ref new) => {
					s.append(&true).append(new);
				}
				None => {
					s.append(&false).append_empty_data();
				}
			}
		}
	}
}

impl RlpDecodable for JournalEntry {
	fn decode<D>(decoder: &D) -> Result<Self, DecoderError> where D: Decoder {
		let rlp = decoder.as_rlp();
		let mut entries = HashMap::new();

		for entry in try!(rlp.at(1)).iter() {
			let acc = try!(entry.val_at(0));
			let maybe = try!(entry.at(1));

			let delta = match try!(maybe.val_at(0)) {
				true => Some(try!(maybe.val_at(1))),
				false => None,
			};

			entries.insert(acc, delta);
		}

		Ok(JournalEntry {
			parent: try!(rlp.val_at(0)),
			entries: entries,
		})
	}
}

// The journal used to store meta info.
// Invariants which must be preserved:
//   - The parent entry of any given journal entry must also be present
//     in the journal, unless it's the canonical base being built off of.
//   - No cyclic entries. There should never be a path from any given entry to
//     itself other than the empty path.
//   - Modifications may only point to entries in the journal.
#[derive(Debug, PartialEq)]
struct Journal {
	// maps era, id pairs to potential canonical meta info.
	entries: BTreeMap<(u64, H256), JournalEntry>,
	// maps address hashes to sets of blocks they were modified at.
	modifications: HashMap<H256, BTreeSet<(u64, H256)>>,
	canon_base: (u64, H256), // the base which the journal builds off of.
}

impl Journal {
	// read the journal from the database, starting from the last committed
	// era.
	fn read_from(db: &Database, col: Option<u32>, base: (u64, H256)) -> Result<Self, String> {
		trace!(target: "meta_db", "loading journal");

		let mut journal = Journal {
			entries: BTreeMap::new(),
			modifications: HashMap::new(),
			canon_base: base,
		};

		let mut era = base.0 + 1;
		while let Some(hashes) = try!(db.get(col, &journal_key(&era))).map(|x| ::rlp::decode::<Vec<H256>>(&x)) {
			let candidates: Result<HashMap<_, _>, String> = hashes.into_iter().map(|hash| {
				let journal_rlp = try!(db.get(col, &id_key(&hash)))
					.expect(&format!("corrupted database: missing journal data for {}.", hash));

				let entry: JournalEntry = ::rlp::decode(&journal_rlp);

				for acc in entry.entries.keys() {
					journal.modifications.entry(*acc).or_insert_with(BTreeSet::new).insert((era, hash));
				}

				Ok(((era, hash), entry))
			}).collect();
			let candidates = try!(candidates);

			trace!(target: "meta_db", "journal: loaded {} candidates for era {}", candidates.len(), era);
			journal.entries.extend(candidates);
			era += 1;
		}

		Ok(journal)
	}

	// write journal era.
	fn write_era(&self, col: Option<u32>, batch: &mut DBTransaction, era: u64) {
		let key = journal_key(&era);
		let candidate_hashes: Vec<_> = self.entries.keys()
			.skip_while(|&&(ref e, _)| e < &era)
			.take_while(|&&(e, _)| e == era)
			.map(|&(_, ref h)| h.clone())
			.collect();

		batch.put(col, &key, &*::rlp::encode(&candidate_hashes));
	}
}

impl HeapSizeOf for Journal {
	fn heap_size_of_children(&self) -> usize {
		self.entries.heap_size_of_children()
			// + self.modifications.heap_size_of_children()
			// ^~~ uncomment when BTreeSet has a HeapSizeOf implementation.
	}
}

/// The account meta-database. See the module docs for more details.
/// It can't be queried without a `MetaBranch` which allows for accurate
/// queries along the current branch.
///
/// This has a short journal period, and is only really usable while syncing.
/// When replaying old transactions, it can't be used reliably.
#[derive(Clone)]
pub struct MetaDB {
	col: Option<u32>,
	db: Arc<Database>,
	journal: Arc<RwLock<Journal>>,
	overlay: HashMap<H256, Option<AccountMeta>>,
}

impl MetaDB {
	/// Create a new `MetaDB` from a database and column. This will also load the journal.
	///
	/// After creation, check the last committed era to see if the genesis state
	/// is in. If not, it should be inserted, journalled, and marked canonical.
	pub fn new(db: Arc<Database>, col: Option<u32>, genesis_hash: &H256) -> Result<Self, String> {
		let base: (u64, H256) = try!(db.get(col, b"base")).map(|raw| {
			let rlp = Rlp::new(&raw);

			(rlp.val_at(0), rlp.val_at(1))
		}).unwrap_or_else(|| (0, genesis_hash.clone()));

		trace!(target: "meta_db", "Creating meta_db with base {:?}", base);

		let journal = try!(Journal::read_from(&*db, col, base));

		Ok(MetaDB {
			col: col,
			db: db,
			journal: Arc::new(RwLock::new(journal)),
			overlay: HashMap::new(),
		})
	}

	/// Journal all pending changes under the given era and id.
	pub fn journal_under(&mut self, batch: &mut DBTransaction, now: u64, id: H256, parent_id: H256) {
		trace!(target: "meta_db", "journalling ({}, {})", now, id);
		let mut journal = self.journal.write();

		let j_entry = JournalEntry {
			parent: parent_id,
			entries: ::std::mem::replace(&mut self.overlay, HashMap::new()),
		};

		if now <= journal.canon_base.0 {
			trace!(target: "meta_db", "ignoring journal request for ancient era: {:?}", (now, id));
			return;
		}

		for acc in j_entry.entries.keys() {
			journal.modifications.entry(*acc).or_insert_with(BTreeSet::new).insert((now, id));
		}

		let encoded = ::rlp::encode(&j_entry);

		trace!(target: "meta_db", "produced entry: {:?}", &*encoded);

		batch.put(self.col, &id_key(&id), &encoded);

		journal.entries.insert((now, id), j_entry);
		journal.write_era(self.col, batch, now);
	}

	/// Mark a candidate for an era as canonical, applying its changes
	/// and invalidating its siblings.
	pub fn mark_canonical(&mut self, batch: &mut DBTransaction, end_era: u64, canon_id: H256) {
		let mut journal = self.journal.write();

		// early exit if this state is before our canonical base.
		if journal.canon_base.0 > end_era { return }

		trace!(target: "meta_db", "mark_canonical: end=({}, {}), cur={:?}", end_era, canon_id, journal.canon_base);

		let candidate_hashes: Vec<_> = journal.entries.keys()
			.skip_while(|&&(ref e, _)| e < &end_era)
			.take_while(|&&(e, _)| e == end_era)
			.map(|&(_, ref h)| h.clone())
			.collect();

		for id in candidate_hashes {
			let entry = journal.entries.remove(&(end_era, id)).expect("entries known to contain this key; qed");
			batch.delete(self.col, &id_key(&id));

			// remove modifications entries.
			for acc in entry.entries.keys() {
				let remove = match journal.modifications.get_mut(acc) {
					Some(ref mut mods) => {
						mods.remove(&(end_era, id));
						mods.is_empty()
					}
					None => false,
				};

				if remove {
					journal.modifications.remove(acc);
				}
			}

			// apply canonical changes.
			if id == canon_id {
				for (acc, delta) in entry.entries {
					match delta {
						Some(delta) => batch.put(self.col, &acc, &*::rlp::encode(&delta)),
						None => batch.delete(self.col, &acc),
					}
				}
			}
		}

		journal.canon_base = (end_era, canon_id);

		// update meta keys in the database.
		let mut base_stream = RlpStream::new_list(2);
		base_stream.append(&journal.canon_base.0).append(&journal.canon_base.1);

		batch.put(self.col, b"base", &*base_stream.drain());
		batch.delete(self.col, &journal_key(&end_era));
	}

	/// Inject the contents of the overlay directly into the backing database,
	/// signifying that they are known to be canonical.
	/// Used in snapshot restoration.
	pub fn inject(&mut self, batch: &mut DBTransaction) {
		for (acc, delta) in self.overlay.drain() {
			match delta {
				Some(delta) => batch.put(self.col, &acc, &*::rlp::encode(&delta)),
				None => batch.delete(self.col, &acc),
			}
		}
	}

	/// Manually update the canonical base.
	/// Should only be done when there is a solid guarantee that
	/// the data in the backing database does, in fact, correspond with
	/// the given canonical base.
	pub fn update_base(&mut self, batch: &mut DBTransaction, era: u64, id: H256) {
		let mut base_stream = RlpStream::new_list(2);
		base_stream.append(&era).append(&id);
		batch.put(self.col, b"base", &*base_stream.drain());

		self.journal.write().canon_base = (era, id);
	}

	/// Query the state of an account at a given block. A return value
	/// of `None` means that the account definitively does not exist on this branch.
	/// This will query the overlay of pending changes first.
	///
	/// Will fail on database error, state pruned, or unexpected missing journal entry.
	pub fn get(&self, address_hash: &H256, at: (u64, H256)) -> Result<Option<AccountMeta>, Error> {
		let get_from_db = || match self.db.get(self.col, &*address_hash) {
			Ok(meta) => Ok(meta.map(|x| ::rlp::decode(&x))),
			Err(e) => Err(Error::Database(e)),
		};

		if let Some(meta) = self.overlay.get(address_hash) {
			return Ok(meta.clone());
		}

		let journal = self.journal.read();
		trace!(target: "meta_db", "get: {:?} at={:?}, base={:?}", address_hash, at, journal.canon_base);

		// fast path for base query.
		if at == journal.canon_base {
			return get_from_db();
		}

		let (mut era, mut id) = at;
		let mut entry = try!(journal.entries.get(&(era, id)).ok_or_else(|| Error::MissingJournalEntry(era, id)));

		// iterate the modifications for this account in reverse order (by id),
		'a:
		for &(mod_era, ref mod_id) in journal.modifications.get(address_hash).into_iter().flat_map(|m| m.iter().rev()) {
			debug_assert!(mod_era > journal.canon_base.0, "modification from pruned entry {:?} still remains in journal", (mod_era, mod_id));

			// walk the relevant path down the journal backwards until we're aligned with
			// the era
			while era > mod_era {
				id = entry.parent;
				era -= 1;

				if era == journal.canon_base.0 { break 'a }
				entry = try!(journal.entries.get(&(era, id)).ok_or_else(|| Error::MissingJournalEntry(era, id)));
			}

			// then continue until we reach the right ID or have to traverse further down.
			if mod_id != &id { continue }

			assert_eq!((era, &id), (mod_era, mod_id), "journal traversal led to wrong entry");
			return Ok(entry.entries.get(address_hash)
				.expect("modifications set always contains correct entries; qed")
				.clone());
		}

		if era <= journal.canon_base.0 && id != journal.canon_base.1 {
			return Err(Error::StatePruned(era, id));
		}

		// no known modifications -- fetch from database.
		get_from_db()
	}

	/// Set the given account's details in the pending changes
	/// overlay.
	/// This will overwrite any previous changes to the overlay,
	/// and will be queried prior to the journal.
	pub fn set(&mut self, address_hash: H256, meta: AccountMeta) {
		trace!(target: "meta_db", "set({:?}, {:?})", address_hash, meta);
		self.overlay.insert(address_hash, Some(meta));
	}

	/// Destroy the account details here.
	pub fn remove(&mut self, address_hash: H256) {
		trace!(target: "meta_db", "remove({:?})", address_hash);
		self.overlay.insert(address_hash, None);
	}
}

impl HeapSizeOf for MetaDB {
	fn heap_size_of_children(&self) -> usize {
		self.overlay.heap_size_of_children() + self.journal.read().heap_size_of_children()
	}
}

#[cfg(test)]
mod tests {
	use super::{AccountMeta, MetaDB};
	use devtools::RandomTempPath;

	use util::{U256, H256, FixedHash};
	use util::kvdb::Database;

	use std::sync::Arc;

	#[test]
	fn loads_journal() {
		let path = RandomTempPath::create_dir();
		let db = Arc::new(Database::open_default(&*path.as_path().to_string_lossy()).unwrap());
		let mut meta_db = MetaDB::new(db.clone(), None, &Default::default()).unwrap();

		for i in 0..10u64 {
			let this = U256::from(i + 1);
			let parent = U256::from(i);

			let mut batch = db.transaction();
			meta_db.journal_under(&mut batch, i + 1, this.into(), parent.into());
			db.write(batch).unwrap();
		}

		let mut batch = db.transaction();
		meta_db.mark_canonical(&mut batch, 1, U256::from(1).into());
		db.write(batch).unwrap();

		let journal = meta_db.journal;
		let meta_db = MetaDB::new(db.clone(), None, &Default::default()).unwrap();

		assert_eq!(&*journal.read(), &*meta_db.journal.read());
	}

	#[test]
	fn query_fork() {
		let path = RandomTempPath::create_dir();
		let db = Arc::new(Database::open_default(&*path.as_path().to_string_lossy()).unwrap());
		let mut meta_db = MetaDB::new(db.clone(), None, &H256::zero()).unwrap();

		let h1 = H256::random();
		let h2a = H256::random();
		let h2b = H256::random();
		let h3a = H256::random();
		let h3b = H256::random();

		let mut new_meta = AccountMeta::default();
		new_meta.balance = new_meta.balance + 5u64.into();

		let acc = H256::random();
		meta_db.set(acc.clone(), AccountMeta::default());

		let mut batch = db.transaction();
		meta_db.journal_under(&mut batch, 1, h1, H256::zero());
		db.write(batch).unwrap();

		// fork side 1 -- deleted in first block.
		{
			meta_db.remove(acc.clone());

			let mut batch = db.transaction();
			meta_db.journal_under(&mut batch, 2, h2a, h1);
			db.write(batch).unwrap();

			let mut batch = db.transaction();
			meta_db.journal_under(&mut batch, 3, h3a, h2a);
			db.write(batch).unwrap();
		}


		// fork side 2: changes in second block.
		{
			let mut batch = db.transaction();
			meta_db.journal_under(&mut batch, 2, h2b, h1);
			db.write(batch).unwrap();

			meta_db.set(acc.clone(), new_meta.clone());

			let mut batch = db.transaction();
			meta_db.journal_under(&mut batch, 3, h3b, h2b);
			db.write(batch).unwrap();
		}

		assert_eq!(meta_db.get(&acc, (1, h1)).unwrap(), Some(AccountMeta::default()));
		assert_eq!(meta_db.get(&acc, (2, h2a)).unwrap(), None);
		assert_eq!(meta_db.get(&acc, (2, h2b)).unwrap(), Some(AccountMeta::default()));
		assert_eq!(meta_db.get(&acc, (3, h3a)).unwrap(), None);
		assert_eq!(meta_db.get(&acc, (3, h3b)).unwrap(), Some(new_meta.clone()));

		let mut batch = db.transaction();
		meta_db.mark_canonical(&mut batch, 1, h1);
		db.write(batch).unwrap();

		assert_eq!(meta_db.get(&acc, (1, h1)).unwrap(), Some(AccountMeta::default()));
		assert_eq!(meta_db.get(&acc, (2, h2a)).unwrap(), None);
		assert_eq!(meta_db.get(&acc, (2, h2b)).unwrap(), Some(AccountMeta::default()));
		assert_eq!(meta_db.get(&acc, (3, h3a)).unwrap(), None);
		assert_eq!(meta_db.get(&acc, (3, h3b)).unwrap(), Some(new_meta.clone()));
	}
}