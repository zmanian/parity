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

//! Rust VM implementation

#[macro_use]
mod informant;
mod gasometer;
mod stack;
mod memory;
mod shared_cache;

use self::gasometer::Gasometer;
use self::stack::{Stack, VecStack};
use self::memory::Memory;
pub use self::shared_cache::SharedCache;

use std::marker::PhantomData;
use action_params::{ActionParams, ActionValue};
use types::executed::CallType;
use evm::instructions::{self, Instruction, InstructionInfo};
use evm::{self, MessageCallResult, ContractCreateResult, CostType, Schedule};
use bit_set::BitSet;

use util::*;

type CodePosition = usize;
type ProgramCounter = usize;

const ONE: U256 = U256([1, 0, 0, 0]);
const TWO: U256 = U256([2, 0, 0, 0]);
const TWO_POW_5: U256 = U256([0x20, 0, 0, 0]);
const TWO_POW_8: U256 = U256([0x100, 0, 0, 0]);
const TWO_POW_16: U256 = U256([0x10000, 0, 0, 0]);
const TWO_POW_24: U256 = U256([0x1000000, 0, 0, 0]);
const TWO_POW_64: U256 = U256([0, 0x1, 0, 0]); // 0x1 00000000 00000000
const TWO_POW_96: U256 = U256([0, 0x100000000, 0, 0]); //0x1 00000000 00000000 00000000
const TWO_POW_224: U256 = U256([0, 0, 0, 0x100000000]); //0x1 00000000 00000000 00000000 00000000 00000000 00000000 00000000
const TWO_POW_248: U256 = U256([0, 0, 0, 0x100000000000000]); //0x1 00000000 00000000 00000000 00000000 00000000 00000000 00000000 000000

/// Abstraction over raw vector of Bytes. Easier state management of PC.
struct CodeReader<'a> {
	position: ProgramCounter,
	code: &'a Bytes
}

#[cfg_attr(feature="dev", allow(len_without_is_empty))]
impl<'a> CodeReader<'a> {

	/// Create new code reader - starting at position 0.
	fn new(code: &'a Bytes) -> Self {
		CodeReader {
			position: 0,
			code: code,
		}
	}

	/// Get `no_of_bytes` from code and convert to U256. Move PC
	fn read(&mut self, no_of_bytes: usize) -> U256 {
		let pos = self.position;
		self.position += no_of_bytes;
		let max = cmp::min(pos + no_of_bytes, self.code.len());
		U256::from(&self.code[pos..max])
	}

	fn len (&self) -> usize {
		self.code.len()
	}
}

enum InstructionResult<Gas> {
	Ok,
	UnusedGas(Gas),
	JumpToPosition(U256),
	// gas left, init_orf, init_size
	StopExecutionNeedsReturn(Gas, U256, U256),
	StopExecution,
}


/// Intepreter EVM implementation
pub struct Interpreter<Cost, Ext> {
	mem: Vec<u8>,
	stack: VecStack<U256>,
	cache: Arc<SharedCache>,
	_gas: PhantomData<Cost>,
	_ext: PhantomData<Ext>,
}

impl<Cost: CostType, Ext: evm::Ext> evm::Evm<Ext> for Interpreter<Cost, Ext> {
	fn exec(&mut self, params: ActionParams, mut ext: Ext) -> evm::Result<U256> {
		self.mem.clear();
		self.stack.clear();
		self.stack.expand(ext.schedule().stack_limit);

		let mut informant = informant::EvmInformant::new(ext.depth());

		let code = &params.code.as_ref().expect("exec always called with code; qed");
		let valid_jump_destinations = self.cache.jump_destinations(&params.code_hash, code);

		let mut gasometer = Gasometer::<Cost>::new(try!(Cost::from_u256(params.gas)));
		let mut reader = CodeReader::new(code);
		let infos = &*instructions::INSTRUCTIONS;

		while reader.position < code.len() {
			let instruction = code[reader.position];
			reader.position += 1;

			let info = &infos[instruction as usize];
			try!(self.verify_instruction(ext.schedule(), instruction, info));

			// Calculate gas cost
			let requirements = try!(gasometer.requirements(&mut ext, instruction, info, &self.stack, self.mem.size()));
			// TODO: make compile-time removable if too much of a performance hit.
			let trace_executed = ext.trace_prepare_execute(reader.position - 1, instruction, &requirements.gas_cost);

			try!(gasometer.verify_gas(&requirements.gas_cost));
			self.mem.expand(requirements.memory_required_size);
			gasometer.current_mem_gas = requirements.memory_total_gas;
			gasometer.current_gas = gasometer.current_gas - requirements.gas_cost;

			evm_debug!({ informant.before_instruction(reader.position, instruction, info, &gasometer.current_gas, &self.stack) });

			let (mem_written, store_written) = match trace_executed {
				true => (Self::mem_written(instruction, &self.stack), Self::store_written(instruction, &self.stack)),
				false => (None, None),
			};

			// Execute instruction
			let result = try!(self.exec_instruction(
				&mut ext, gasometer.current_gas, &params, instruction, &mut reader, requirements.provide_gas
			));

			evm_debug!({ informant.after_instruction(instruction) });

			if let InstructionResult::UnusedGas(ref gas) = result {
				gasometer.current_gas = gasometer.current_gas + *gas;
			}

			if trace_executed {
				let stack = self.stack.peek_top(info.ret);
				let mem = mem_written.map(|(o, s)| (o, &(self.mem[o..(o + s)])));
				ext.trace_executed(gasometer.current_gas.as_u256(), stack, mem, store_written);
			}

			// Advance
			match result {
				InstructionResult::JumpToPosition(position) => {
					let pos = try!(self.verify_jump(position, &valid_jump_destinations));
					reader.position = pos;
				},
				InstructionResult::StopExecutionNeedsReturn(gas, off, size) => {
					informant.done();

					let slice = self.mem.read_slice(off, size);
					return ext.ret(&gas.as_u256(), slice);
				},
				InstructionResult::StopExecution => break,
				_ => {},
			}
		}
		informant.done();
		Ok(gasometer.current_gas.as_u256())
	}
}

impl<Cost: CostType, Ext: evm::Ext> Interpreter<Cost, Ext> {
	/// Create a new `Interpreter` instance with shared cache.
	pub fn new(cache: Arc<SharedCache>) -> Self {
		Interpreter {
			mem: Vec::new(),
			stack: VecStack::with_capacity(1024, U256::zero()),
			cache: cache,
			_gas: PhantomData,
			_ext: PhantomData,
		}
	}

	fn verify_instruction(&self, schedule: &Schedule, instruction: Instruction, info: &InstructionInfo) -> evm::Result<()> {

		if !schedule.have_delegate_call && instruction == instructions::DELEGATECALL {
			return Err(evm::Error::BadInstruction {
				instruction: instruction
			});
		}

		if info.tier == instructions::GasPriceTier::Invalid {
			return Err(evm::Error::BadInstruction {
				instruction: instruction
			});
		}

		if !self.stack.has(info.args) {
			Err(evm::Error::StackUnderflow {
				instruction: info.name,
				wanted: info.args,
				on_stack: self.stack.size()
			})
		} else if self.stack.size() - info.args + info.ret > schedule.stack_limit {
			Err(evm::Error::OutOfStack {
				instruction: info.name,
				wanted: info.ret - info.args,
				limit: schedule.stack_limit
			})
		} else {
			Ok(())
		}
	}

	fn mem_written(
		instruction: Instruction,
		stack: &Stack<U256>
	) -> Option<(usize, usize)> {
		match instruction {
			instructions::MSTORE | instructions::MLOAD => Some((stack.peek(0).low_u64() as usize, 32)),
			instructions::MSTORE8 => Some((stack.peek(0).low_u64() as usize, 1)),
			instructions::CALLDATACOPY | instructions::CODECOPY => Some((stack.peek(0).low_u64() as usize, stack.peek(2).low_u64() as usize)),
			instructions::EXTCODECOPY => Some((stack.peek(1).low_u64() as usize, stack.peek(3).low_u64() as usize)),
			instructions::CALL | instructions::CALLCODE => Some((stack.peek(5).low_u64() as usize, stack.peek(6).low_u64() as usize)),
			instructions::DELEGATECALL => Some((stack.peek(4).low_u64() as usize, stack.peek(5).low_u64() as usize)),
			_ => None,
		}
	}

	fn store_written(
		instruction: Instruction,
		stack: &Stack<U256>
	) -> Option<(U256, U256)> {
		match instruction {
			instructions::SSTORE => Some((stack.peek(0).clone(), stack.peek(1).clone())),
			_ => None,
		}
	}

	#[cfg_attr(feature="dev", allow(too_many_arguments))]
	fn exec_instruction(
		&mut self,
		ext: &mut Ext,
		gas: Cost,
		params: &ActionParams,
		instruction: Instruction,
		code: &mut CodeReader,
		provided: Option<Cost>
	) -> evm::Result<InstructionResult<Cost>> {
		match instruction {
			instructions::JUMP => {
				let jump = self.stack.pop_back();
				return Ok(InstructionResult::JumpToPosition(
					jump
				));
			},
			instructions::JUMPI => {
				let jump = self.stack.pop_back();
				let condition = self.stack.pop_back();
				if !condition.is_zero() {
					return Ok(InstructionResult::JumpToPosition(
						jump
					));
				}
			},
			instructions::JUMPDEST => {
				// ignore
			},
			instructions::CREATE => {
				let endowment = self.stack.pop_back();
				let init_off = self.stack.pop_back();
				let init_size = self.stack.pop_back();
				let create_gas = provided.expect("`provided` comes through Self::exec from `Gasometer::get_gas_cost_mem`; `gas_gas_mem_cost` guarantees `Some` when instruction is `CALL`/`CALLCODE`/`DELEGATECALL`/`CREATE`; this is `CREATE`; qed");

				let contract_code = self.mem.read_slice(init_off, init_size);
				let can_create = ext.balance(&params.address) >= endowment && ext.depth() < ext.schedule().max_depth;

				if !can_create {
					self.stack.push(U256::zero());
					return Ok(InstructionResult::UnusedGas(create_gas));
				}

				let create_result = ext.create(&create_gas.as_u256(), &endowment, contract_code);
				return match create_result {
					ContractCreateResult::Created(address, gas_left) => {
						self.stack.push(address_to_u256(address));
						Ok(InstructionResult::UnusedGas(Cost::from_u256(gas_left).expect("Gas left cannot be greater.")))
					},
					ContractCreateResult::Failed => {
						self.stack.push(U256::zero());
						Ok(InstructionResult::Ok)
					}
				};
			},
			instructions::CALL | instructions::CALLCODE | instructions::DELEGATECALL => {
				assert!(ext.schedule().call_value_transfer_gas > ext.schedule().call_stipend, "overflow possible");
				self.stack.pop_back();
				let call_gas = provided.expect("`provided` comes through Self::exec from `Gasometer::get_gas_cost_mem`; `gas_gas_mem_cost` guarantees `Some` when instruction is `CALL`/`CALLCODE`/`DELEGATECALL`/`CREATE`; this is one of `CALL`/`CALLCODE`/`DELEGATECALL`; qed");
				let code_address = self.stack.pop_back();
				let code_address = u256_to_address(&code_address);

				let value = if instruction == instructions::DELEGATECALL {
					None
				} else {
					Some(self.stack.pop_back())
				};

				let in_off = self.stack.pop_back();
				let in_size = self.stack.pop_back();
				let out_off = self.stack.pop_back();
				let out_size = self.stack.pop_back();

				// Add stipend (only CALL|CALLCODE when value > 0)
				let call_gas = call_gas + value.map_or_else(|| Cost::from(0), |val| match val.is_zero() {
					false => Cost::from(ext.schedule().call_stipend),
					true => Cost::from(0),
				});

				// Get sender & receive addresses, check if we have balance
				let (sender_address, receive_address, has_balance, call_type) = match instruction {
					instructions::CALL => {
						let has_balance = ext.balance(&params.address) >= value.expect("value set for all but delegate call; qed");
						(&params.address, &code_address, has_balance, CallType::Call)
					},
					instructions::CALLCODE => {
						let has_balance = ext.balance(&params.address) >= value.expect("value set for all but delegate call; qed");
						(&params.address, &params.address, has_balance, CallType::CallCode)
					},
					instructions::DELEGATECALL => (&params.sender, &params.address, true, CallType::DelegateCall),
					_ => panic!(format!("Unexpected instruction {} in CALL branch.", instruction))
				};

				let can_call = has_balance && ext.depth() < ext.schedule().max_depth;
				if !can_call {
					self.stack.push(U256::zero());
					return Ok(InstructionResult::UnusedGas(call_gas));
				}

				let call_result = {
					// we need to write and read from memory in the same time
					// and we don't want to copy
					let input = unsafe { ::std::mem::transmute(self.mem.read_slice(in_off, in_size)) };
					let output = self.mem.writeable_slice(out_off, out_size);
					ext.call(&call_gas.as_u256(), sender_address, receive_address, value, input, &code_address, output, call_type)
				};

				return match call_result {
					MessageCallResult::Success(gas_left) => {
						self.stack.push(U256::one());
						Ok(InstructionResult::UnusedGas(Cost::from_u256(gas_left).expect("Gas left cannot be greater then current one")))
					},
					MessageCallResult::Failed  => {
						self.stack.push(U256::zero());
						Ok(InstructionResult::Ok)
					}
				};
			},
			instructions::RETURN => {
				let init_off = self.stack.pop_back();
				let init_size = self.stack.pop_back();

				return Ok(InstructionResult::StopExecutionNeedsReturn(gas, init_off, init_size))
			},
			instructions::STOP => {
				return Ok(InstructionResult::StopExecution);
			},
			instructions::SUICIDE => {
				let address = self.stack.pop_back();
				ext.suicide(&u256_to_address(&address));
				return Ok(InstructionResult::StopExecution);
			},
			instructions::LOG0...instructions::LOG4 => {
				let no_of_topics = instructions::get_log_topics(instruction);

				let offset = self.stack.pop_back();
				let size = self.stack.pop_back();
				let topics = self.stack.pop_n(no_of_topics)
					.iter()
					.map(H256::from)
					.collect();
				ext.log(topics, self.mem.read_slice(offset, size));
			},
			instructions::PUSH1...instructions::PUSH32 => {
				let bytes = instructions::get_push_bytes(instruction);
				let val = code.read(bytes);
				self.stack.push(val);
			},
			instructions::MLOAD => {
				let word = self.mem.read(self.stack.pop_back());
				self.stack.push(U256::from(word));
			},
			instructions::MSTORE => {
				let offset = self.stack.pop_back();
				let word = self.stack.pop_back();
				Memory::write(&mut self.mem, offset, word);
			},
			instructions::MSTORE8 => {
				let offset = self.stack.pop_back();
				let byte = self.stack.pop_back();
				self.mem.write_byte(offset, byte);
			},
			instructions::MSIZE => {
				self.stack.push(U256::from(self.mem.size()));
			},
			instructions::SHA3 => {
				let offset = self.stack.pop_back();
				let size = self.stack.pop_back();
				let sha3 = self.mem.read_slice(offset, size).sha3();
				self.stack.push(U256::from(&*sha3));
			},
			instructions::SLOAD => {
				let key = H256::from(&self.stack.pop_back());
				let word = U256::from(&*ext.storage_at(&key));
				self.stack.push(word);
			},
			instructions::SSTORE => {
				let address = H256::from(&self.stack.pop_back());
				let val = self.stack.pop_back();

				let current_val = U256::from(&*ext.storage_at(&address));
				// Increase refund for clear
				if !current_val.is_zero() && val.is_zero() {
					ext.inc_sstore_clears();
				}
				ext.set_storage(address, H256::from(&val));
			},
			instructions::PC => {
				self.stack.push(U256::from(code.position - 1));
			},
			instructions::GAS => {
				self.stack.push(gas.as_u256());
			},
			instructions::ADDRESS => {
				self.stack.push(address_to_u256(params.address.clone()));
			},
			instructions::ORIGIN => {
				self.stack.push(address_to_u256(params.origin.clone()));
			},
			instructions::BALANCE => {
				let address = u256_to_address(&self.stack.pop_back());
				let balance = ext.balance(&address);
				self.stack.push(balance);
			},
			instructions::CALLER => {
				self.stack.push(address_to_u256(params.sender.clone()));
			},
			instructions::CALLVALUE => {
				self.stack.push(match params.value {
					ActionValue::Transfer(val) | ActionValue::Apparent(val) => val
				});
			},
			instructions::CALLDATALOAD => {
				let big_id = self.stack.pop_back();
				let id = big_id.low_u64() as usize;
				let max = id.wrapping_add(32);
				if let Some(data) = params.data.as_ref() {
					let bound = cmp::min(data.len(), max);
					if id < bound && big_id < U256::from(data.len()) {
						let mut v = [0u8; 32];
						v[0..bound-id].clone_from_slice(&data[id..bound]);
						self.stack.push(U256::from(&v[..]))
					} else {
						self.stack.push(U256::zero())
					}
				} else {
					self.stack.push(U256::zero())
				}
			},
			instructions::CALLDATASIZE => {
				self.stack.push(U256::from(params.data.clone().map_or(0, |l| l.len())));
			},
			instructions::CODESIZE => {
				self.stack.push(U256::from(code.len()));
			},
			instructions::EXTCODESIZE => {
				let address = u256_to_address(&self.stack.pop_back());
				let len = ext.extcodesize(&address);
				self.stack.push(U256::from(len));
			},
			instructions::CALLDATACOPY => {
				self.copy_data_to_memory(params.data.as_ref().map_or_else(|| &[] as &[u8], |d| &*d as &[u8]));
			},
			instructions::CODECOPY => {
				self.copy_data_to_memory(params.code.as_ref().map_or_else(|| &[] as &[u8], |c| &**c as &[u8]));
			},
			instructions::EXTCODECOPY => {
				let address = u256_to_address(&self.stack.pop_back());
				let code = ext.extcode(&address);
				self.copy_data_to_memory(&code);
			},
			instructions::GASPRICE => {
				self.stack.push(params.gas_price.clone());
			},
			instructions::BLOCKHASH => {
				let block_number = self.stack.pop_back();
				let block_hash = ext.blockhash(&block_number);
				self.stack.push(U256::from(&*block_hash));
			},
			instructions::COINBASE => {
				self.stack.push(address_to_u256(ext.env_info().author.clone()));
			},
			instructions::TIMESTAMP => {
				self.stack.push(U256::from(ext.env_info().timestamp));
			},
			instructions::NUMBER => {
				self.stack.push(U256::from(ext.env_info().number));
			},
			instructions::DIFFICULTY => {
				self.stack.push(ext.env_info().difficulty.clone());
			},
			instructions::GASLIMIT => {
				self.stack.push(ext.env_info().gas_limit.clone());
			},
			_ => {
				try!(self.exec_stack_instruction(instruction));
			}
		};
		Ok(InstructionResult::Ok)
	}

	fn copy_data_to_memory(&mut self, source: &[u8]) {
		let dest_offset = self.stack.pop_back();
		let source_offset = self.stack.pop_back();
		let size = self.stack.pop_back();
		let source_size = U256::from(source.len());

		let output_end = match source_offset > source_size || size > source_size || source_offset + size > source_size {
			true => {
				let zero_slice = if source_offset > source_size {
					self.mem.writeable_slice(dest_offset, size)
				} else {
					self.mem.writeable_slice(dest_offset + source_size - source_offset, source_offset + size - source_size)
				};
				for i in zero_slice.iter_mut() {
					*i = 0;
				}
				source.len()
			},
			false => (size.low_u64() + source_offset.low_u64()) as usize
		};

		if source_offset < source_size {
			let output_begin = source_offset.low_u64() as usize;
			self.mem.write_slice(dest_offset, &source[output_begin..output_end]);
		}
	}

	fn verify_jump(&self, jump_u: U256, valid_jump_destinations: &BitSet) -> evm::Result<usize> {
		let jump = jump_u.low_u64() as usize;

		if valid_jump_destinations.contains(jump) && U256::from(jump) == jump_u {
			Ok(jump)
		} else {
			Err(evm::Error::BadJumpDestination {
				destination: jump
			})
		}
	}

	fn exec_stack_instruction(&mut self, instruction: Instruction) -> evm::Result<()> {
		match instruction {
			instructions::DUP1...instructions::DUP16 => {
				let position = instructions::get_dup_position(instruction);
				let val = self.stack.peek(position).clone();
				self.stack.push(val);
			},
			instructions::SWAP1...instructions::SWAP16 => {
				let position = instructions::get_swap_position(instruction);
				self.stack.swap_with_top(position)
			},
			instructions::POP => {
				self.stack.pop_back();
			},
			instructions::ADD => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(a.overflowing_add(b).0);
			},
			instructions::MUL => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(a.overflowing_mul(b).0);
			},
			instructions::SUB => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(a.overflowing_sub(b).0);
			},
			instructions::DIV => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(if !b.is_zero() {
					match b {
						ONE => a,
						TWO => a >> 1,
						TWO_POW_5 => a >> 5,
						TWO_POW_8 => a >> 8,
						TWO_POW_16 => a >> 16,
						TWO_POW_24 => a >> 24,
						TWO_POW_64 => a >> 64,
						TWO_POW_96 => a >> 96,
						TWO_POW_224 => a >> 224,
						TWO_POW_248 => a >> 248,
						_ => a.overflowing_div(b).0,
					}
				} else {
					U256::zero()
				});
			},
			instructions::MOD => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(if !b.is_zero() {
					a.overflowing_rem(b).0
				} else {
					U256::zero()
				});
			},
			instructions::SDIV => {
				let (a, sign_a) = get_and_reset_sign(self.stack.pop_back());
				let (b, sign_b) = get_and_reset_sign(self.stack.pop_back());

				// -2^255
				let min = (U256::one() << 255) - U256::one();
				self.stack.push(if b.is_zero() {
					U256::zero()
				} else if a == min && b == !U256::zero() {
					min
				} else {
					let c = a.overflowing_div(b).0;
					set_sign(c, sign_a ^ sign_b)
				});
			},
			instructions::SMOD => {
				let ua = self.stack.pop_back();
				let ub = self.stack.pop_back();
				let (a, sign_a) = get_and_reset_sign(ua);
				let b = get_and_reset_sign(ub).0;

				self.stack.push(if !b.is_zero() {
					let c = a.overflowing_rem(b).0;
					set_sign(c, sign_a)
				} else {
					U256::zero()
				});
			},
			instructions::EXP => {
				let base = self.stack.pop_back();
				let expon = self.stack.pop_back();
				let res = base.overflowing_pow(expon).0;
				self.stack.push(res);
			},
			instructions::NOT => {
				let a = self.stack.pop_back();
				self.stack.push(!a);
			},
			instructions::LT => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(bool_to_u256(a < b));
			},
			instructions::SLT => {
				let (a, neg_a) = get_and_reset_sign(self.stack.pop_back());
				let (b, neg_b) = get_and_reset_sign(self.stack.pop_back());

				let is_positive_lt = a < b && !(neg_a | neg_b);
				let is_negative_lt = a > b && (neg_a & neg_b);
				let has_different_signs = neg_a && !neg_b;

				self.stack.push(bool_to_u256(is_positive_lt | is_negative_lt | has_different_signs));
			},
			instructions::GT => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(bool_to_u256(a > b));
			},
			instructions::SGT => {
				let (a, neg_a) = get_and_reset_sign(self.stack.pop_back());
				let (b, neg_b) = get_and_reset_sign(self.stack.pop_back());

				let is_positive_gt = a > b && !(neg_a | neg_b);
				let is_negative_gt = a < b && (neg_a & neg_b);
				let has_different_signs = !neg_a && neg_b;

				self.stack.push(bool_to_u256(is_positive_gt | is_negative_gt | has_different_signs));
			},
			instructions::EQ => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(bool_to_u256(a == b));
			},
			instructions::ISZERO => {
				let a = self.stack.pop_back();
				self.stack.push(bool_to_u256(a.is_zero()));
			},
			instructions::AND => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(a & b);
			},
			instructions::OR => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(a | b);
			},
			instructions::XOR => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				self.stack.push(a ^ b);
			},
			instructions::BYTE => {
				let word = self.stack.pop_back();
				let val = self.stack.pop_back();
				let byte = match word < U256::from(32) {
					true => (val >> (8 * (31 - word.low_u64() as usize))) & U256::from(0xff),
					false => U256::zero()
				};
				self.stack.push(byte);
			},
			instructions::ADDMOD => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				let c = self.stack.pop_back();

				self.stack.push(if !c.is_zero() {
					// upcast to 512
					let a5 = U512::from(a);
					let res = a5.overflowing_add(U512::from(b)).0;
					let x = res.overflowing_rem(U512::from(c)).0;
					U256::from(x)
				} else {
					U256::zero()
				});
			},
			instructions::MULMOD => {
				let a = self.stack.pop_back();
				let b = self.stack.pop_back();
				let c = self.stack.pop_back();

				self.stack.push(if !c.is_zero() {
					let a5 = U512::from(a);
					let res = a5.overflowing_mul(U512::from(b)).0;
					let x = res.overflowing_rem(U512::from(c)).0;
					U256::from(x)
				} else {
					U256::zero()
				});
			},
			instructions::SIGNEXTEND => {
				let bit = self.stack.pop_back();
				if bit < U256::from(32) {
					let number = self.stack.pop_back();
					let bit_position = (bit.low_u64() * 8 + 7) as usize;

					let bit = number.bit(bit_position);
					let mask = (U256::one() << bit_position) - U256::one();
					self.stack.push(if bit {
						number | !mask
					} else {
						number & mask
					});
				}
			},
			_ => {
				return Err(evm::Error::BadInstruction {
					instruction: instruction
				});
			}
		}
		Ok(())
	}

}

#[inline]
fn get_and_reset_sign(value: U256) -> (U256, bool) {
	let U256(arr) = value;
	let sign = arr[3].leading_zeros() == 0;
	(set_sign(value, sign), sign)
}

#[inline]
fn set_sign(value: U256, sign: bool) -> U256 {
	if sign {
		(!U256::zero() ^ value).overflowing_add(U256::one()).0
	} else {
		value
	}
}

#[inline]
fn bool_to_u256(val: bool) -> U256 {
	if val {
		U256::one()
	} else {
		U256::zero()
	}
}

#[inline]
fn u256_to_address(value: &U256) -> Address {
	Address::from(H256::from(value))
}

#[inline]
fn address_to_u256(value: Address) -> U256 {
	U256::from(&*H256::from(value))
}

