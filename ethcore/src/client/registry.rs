// Autogenerated from JSON contract definition using Rust contract convertor.

use std::string::String;
use std::result::Result;
use std::fmt;
use {util, ethabi};
use util::FixedHash;
use util::Uint;

pub struct Registry {
	contract: ethabi::Contract,
	pub address: util::Address,
	do_call: Box<Fn(util::Address, Vec<u8>) -> Result<Vec<u8>, String> + Send + 'static>,
}
impl Registry {
	pub fn new<F>(address: util::Address, do_call: F) -> Self where F: Fn(util::Address, Vec<u8>) -> Result<Vec<u8>, String> + Send + 'static {
		Registry {
			contract: ethabi::Contract::new(ethabi::Interface::load(b"[{\"constant\":false,\"inputs\":[{\"name\":\"_new\",\"type\":\"address\"}],\"name\":\"setOwner\",\"outputs\":[],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"_name\",\"type\":\"string\"}],\"name\":\"confirmReverse\",\"outputs\":[{\"name\":\"success\",\"type\":\"bool\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"}],\"name\":\"reserve\",\"outputs\":[{\"name\":\"success\",\"type\":\"bool\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"},{\"name\":\"_key\",\"type\":\"string\"},{\"name\":\"_value\",\"type\":\"bytes32\"}],\"name\":\"set\",\"outputs\":[{\"name\":\"success\",\"type\":\"bool\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"}],\"name\":\"drop\",\"outputs\":[{\"name\":\"success\",\"type\":\"bool\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":true,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"},{\"name\":\"_key\",\"type\":\"string\"}],\"name\":\"getAddress\",\"outputs\":[{\"name\":\"\",\"type\":\"address\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"_amount\",\"type\":\"uint256\"}],\"name\":\"setFee\",\"outputs\":[],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"},{\"name\":\"_to\",\"type\":\"address\"}],\"name\":\"transfer\",\"outputs\":[{\"name\":\"success\",\"type\":\"bool\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":true,\"inputs\":[],\"name\":\"owner\",\"outputs\":[{\"name\":\"\",\"type\":\"address\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":true,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"}],\"name\":\"reserved\",\"outputs\":[{\"name\":\"reserved\",\"type\":\"bool\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[],\"name\":\"drain\",\"outputs\":[],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"_name\",\"type\":\"string\"},{\"name\":\"_who\",\"type\":\"address\"}],\"name\":\"proposeReverse\",\"outputs\":[{\"name\":\"success\",\"type\":\"bool\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":true,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"},{\"name\":\"_key\",\"type\":\"string\"}],\"name\":\"getUint\",\"outputs\":[{\"name\":\"\",\"type\":\"uint256\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":true,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"},{\"name\":\"_key\",\"type\":\"string\"}],\"name\":\"get\",\"outputs\":[{\"name\":\"\",\"type\":\"bytes32\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":true,\"inputs\":[],\"name\":\"fee\",\"outputs\":[{\"name\":\"\",\"type\":\"uint256\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":true,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"}],\"name\":\"getOwner\",\"outputs\":[{\"name\":\"\",\"type\":\"address\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":true,\"inputs\":[{\"name\":\"\",\"type\":\"address\"}],\"name\":\"reverse\",\"outputs\":[{\"name\":\"\",\"type\":\"string\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"},{\"name\":\"_key\",\"type\":\"string\"},{\"name\":\"_value\",\"type\":\"uint256\"}],\"name\":\"setUint\",\"outputs\":[{\"name\":\"success\",\"type\":\"bool\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[],\"name\":\"removeReverse\",\"outputs\":[],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"_name\",\"type\":\"bytes32\"},{\"name\":\"_key\",\"type\":\"string\"},{\"name\":\"_value\",\"type\":\"address\"}],\"name\":\"setAddress\",\"outputs\":[{\"name\":\"success\",\"type\":\"bool\"}],\"payable\":false,\"type\":\"function\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":false,\"name\":\"amount\",\"type\":\"uint256\"}],\"name\":\"Drained\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":false,\"name\":\"amount\",\"type\":\"uint256\"}],\"name\":\"FeeChanged\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"name\":\"name\",\"type\":\"bytes32\"},{\"indexed\":true,\"name\":\"owner\",\"type\":\"address\"}],\"name\":\"Reserved\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"name\":\"name\",\"type\":\"bytes32\"},{\"indexed\":true,\"name\":\"oldOwner\",\"type\":\"address\"},{\"indexed\":true,\"name\":\"newOwner\",\"type\":\"address\"}],\"name\":\"Transferred\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"name\":\"name\",\"type\":\"bytes32\"},{\"indexed\":true,\"name\":\"owner\",\"type\":\"address\"}],\"name\":\"Dropped\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"name\":\"name\",\"type\":\"bytes32\"},{\"indexed\":true,\"name\":\"owner\",\"type\":\"address\"},{\"indexed\":true,\"name\":\"key\",\"type\":\"string\"},{\"indexed\":false,\"name\":\"plainKey\",\"type\":\"string\"}],\"name\":\"DataChanged\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"name\":\"name\",\"type\":\"string\"},{\"indexed\":true,\"name\":\"reverse\",\"type\":\"address\"}],\"name\":\"ReverseProposed\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"name\":\"name\",\"type\":\"string\"},{\"indexed\":true,\"name\":\"reverse\",\"type\":\"address\"}],\"name\":\"ReverseConfirmed\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"name\":\"name\",\"type\":\"string\"},{\"indexed\":true,\"name\":\"reverse\",\"type\":\"address\"}],\"name\":\"ReverseRemoved\",\"type\":\"event\"},{\"anonymous\":false,\"inputs\":[{\"indexed\":true,\"name\":\"old\",\"type\":\"address\"},{\"indexed\":true,\"name\":\"current\",\"type\":\"address\"}],\"name\":\"NewOwner\",\"type\":\"event\"}]").expect("JSON is autogenerated; qed")),
			address: address,
			do_call: Box::new(do_call),
		}
	}
	fn as_string<T: fmt::Debug>(e: T) -> String { format!("{:?}", e) }
	
	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_new","type":"address"}],"name":"setOwner","outputs":[],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn set_owner(&self, _new: &util::Address) -> Result<(), String> { 
		let call = self.contract.function("setOwner".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::Address(_new.clone().0)]
		).map_err(Self::as_string)?;
		call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		
		Ok(()) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_name","type":"string"}],"name":"confirmReverse","outputs":[{"name":"success","type":"bool"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn confirm_reverse(&self, _name: &str) -> Result<bool, String> { 
		let call = self.contract.function("confirmReverse".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::String(_name.to_owned())]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bool().ok_or("Invalid type returned")?; r })) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_name","type":"bytes32"}],"name":"reserve","outputs":[{"name":"success","type":"bool"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn reserve(&self, _name: &util::H256) -> Result<bool, String> { 
		let call = self.contract.function("reserve".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned())]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bool().ok_or("Invalid type returned")?; r })) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_name","type":"bytes32"},{"name":"_key","type":"string"},{"name":"_value","type":"bytes32"}],"name":"set","outputs":[{"name":"success","type":"bool"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn set(&self, _name: &util::H256, _key: &str, _value: &util::H256) -> Result<bool, String> { 
		let call = self.contract.function("set".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned()), ethabi::Token::String(_key.to_owned()), ethabi::Token::FixedBytes(_value.as_ref().to_owned())]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bool().ok_or("Invalid type returned")?; r })) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_name","type":"bytes32"}],"name":"drop","outputs":[{"name":"success","type":"bool"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn drop(&self, _name: &util::H256) -> Result<bool, String> { 
		let call = self.contract.function("drop".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned())]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bool().ok_or("Invalid type returned")?; r })) 
	}

	/// Auto-generated from: `{"constant":true,"inputs":[{"name":"_name","type":"bytes32"},{"name":"_key","type":"string"}],"name":"getAddress","outputs":[{"name":"","type":"address"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn get_address(&self, _name: &util::H256, _key: &str) -> Result<util::Address, String> { 
		let call = self.contract.function("getAddress".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned()), ethabi::Token::String(_key.to_owned())]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_address().ok_or("Invalid type returned")?; util::Address::from(r) })) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_amount","type":"uint256"}],"name":"setFee","outputs":[],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn set_fee(&self, _amount: util::U256) -> Result<(), String> { 
		let call = self.contract.function("setFee".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::Uint({ let mut r = [0u8; 32]; _amount.to_big_endian(&mut r); r })]
		).map_err(Self::as_string)?;
		call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		
		Ok(()) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_name","type":"bytes32"},{"name":"_to","type":"address"}],"name":"transfer","outputs":[{"name":"success","type":"bool"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn transfer(&self, _name: &util::H256, _to: &util::Address) -> Result<bool, String> { 
		let call = self.contract.function("transfer".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned()), ethabi::Token::Address(_to.clone().0)]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bool().ok_or("Invalid type returned")?; r })) 
	}

	/// Auto-generated from: `{"constant":true,"inputs":[],"name":"owner","outputs":[{"name":"","type":"address"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn owner(&self) -> Result<util::Address, String> { 
		let call = self.contract.function("owner".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_address().ok_or("Invalid type returned")?; util::Address::from(r) })) 
	}

	/// Auto-generated from: `{"constant":true,"inputs":[{"name":"_name","type":"bytes32"}],"name":"reserved","outputs":[{"name":"reserved","type":"bool"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn reserved(&self, _name: &util::H256) -> Result<bool, String> { 
		let call = self.contract.function("reserved".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned())]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bool().ok_or("Invalid type returned")?; r })) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[],"name":"drain","outputs":[],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn drain(&self) -> Result<(), String> { 
		let call = self.contract.function("drain".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![]
		).map_err(Self::as_string)?;
		call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		
		Ok(()) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_name","type":"string"},{"name":"_who","type":"address"}],"name":"proposeReverse","outputs":[{"name":"success","type":"bool"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn propose_reverse(&self, _name: &str, _who: &util::Address) -> Result<bool, String> { 
		let call = self.contract.function("proposeReverse".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::String(_name.to_owned()), ethabi::Token::Address(_who.clone().0)]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bool().ok_or("Invalid type returned")?; r })) 
	}

	/// Auto-generated from: `{"constant":true,"inputs":[{"name":"_name","type":"bytes32"},{"name":"_key","type":"string"}],"name":"getUint","outputs":[{"name":"","type":"uint256"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn get_uint(&self, _name: &util::H256, _key: &str) -> Result<util::U256, String> { 
		let call = self.contract.function("getUint".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned()), ethabi::Token::String(_key.to_owned())]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_uint().ok_or("Invalid type returned")?; util::U256::from(r.as_ref()) })) 
	}

	/// Auto-generated from: `{"constant":true,"inputs":[{"name":"_name","type":"bytes32"},{"name":"_key","type":"string"}],"name":"get","outputs":[{"name":"","type":"bytes32"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn get(&self, _name: &util::H256, _key: &str) -> Result<util::H256, String> { 
		let call = self.contract.function("get".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned()), ethabi::Token::String(_key.to_owned())]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_fixed_bytes().ok_or("Invalid type returned")?; util::H256::from_slice(r.as_ref()) })) 
	}

	/// Auto-generated from: `{"constant":true,"inputs":[],"name":"fee","outputs":[{"name":"","type":"uint256"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn fee(&self) -> Result<util::U256, String> { 
		let call = self.contract.function("fee".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_uint().ok_or("Invalid type returned")?; util::U256::from(r.as_ref()) })) 
	}

	/// Auto-generated from: `{"constant":true,"inputs":[{"name":"_name","type":"bytes32"}],"name":"getOwner","outputs":[{"name":"","type":"address"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn get_owner(&self, _name: &util::H256) -> Result<util::Address, String> { 
		let call = self.contract.function("getOwner".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned())]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_address().ok_or("Invalid type returned")?; util::Address::from(r) })) 
	}

	/// Auto-generated from: `{"constant":true,"inputs":[{"name":"","type":"address"}],"name":"reverse","outputs":[{"name":"","type":"string"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn reverse(&self, _1: &util::Address) -> Result<String, String> { 
		let call = self.contract.function("reverse".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::Address(_1.clone().0)]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_string().ok_or("Invalid type returned")?; r })) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_name","type":"bytes32"},{"name":"_key","type":"string"},{"name":"_value","type":"uint256"}],"name":"setUint","outputs":[{"name":"success","type":"bool"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn set_uint(&self, _name: &util::H256, _key: &str, _value: util::U256) -> Result<bool, String> { 
		let call = self.contract.function("setUint".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned()), ethabi::Token::String(_key.to_owned()), ethabi::Token::Uint({ let mut r = [0u8; 32]; _value.to_big_endian(&mut r); r })]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bool().ok_or("Invalid type returned")?; r })) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[],"name":"removeReverse","outputs":[],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn remove_reverse(&self) -> Result<(), String> { 
		let call = self.contract.function("removeReverse".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![]
		).map_err(Self::as_string)?;
		call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		
		Ok(()) 
	}

	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"_name","type":"bytes32"},{"name":"_key","type":"string"},{"name":"_value","type":"address"}],"name":"setAddress","outputs":[{"name":"success","type":"bool"}],"payable":false,"type":"function"}`
	#[allow(dead_code)]
	pub fn set_address(&self, _name: &util::H256, _key: &str, _value: &util::Address) -> Result<bool, String> { 
		let call = self.contract.function("setAddress".into()).map_err(Self::as_string)?;
		let data = call.encode_call(
			vec![ethabi::Token::FixedBytes(_name.as_ref().to_owned()), ethabi::Token::String(_key.to_owned()), ethabi::Token::Address(_value.clone().0)]
		).map_err(Self::as_string)?;
		let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
		let mut result = output.into_iter().rev().collect::<Vec<_>>();
		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bool().ok_or("Invalid type returned")?; r })) 
	}
}