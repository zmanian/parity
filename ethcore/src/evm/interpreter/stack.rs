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

use evm::instructions;

/// Stack trait with VM-friendly API
pub trait Stack<T> {
	/// Returns `Stack[len(Stack) - no_from_top]`
	fn peek(&self, no_from_top: usize) -> &T;
	/// Swaps Stack[len(Stack)] and Stack[len(Stack) - no_from_top]
	fn swap_with_top(&mut self, no_from_top: usize);
	/// Returns true if Stack has at least `no_of_elems` elements
	fn has(&self, no_of_elems: usize) -> bool;
	/// Get element from top and remove it from Stack. Panics if stack is empty.
	fn pop_back(&mut self) -> T;
	/// Get (up to `instructions::MAX_NO_OF_TOPICS`) elements from top and remove them from Stack. Panics if stack is empty.
	fn pop_n(&mut self, no_of_elems: usize) -> &[T];
	/// Add element on top of the Stack
	fn push(&mut self, elem: T);
	/// Get number of elements on Stack
	fn size(&self) -> usize;
	/// Returns all data on stack.
	fn peek_top(&self, no_of_elems: usize) -> &[T];
	/// Clears the stack
	fn clear(&mut self);
}

#[derive(Default)]
pub struct VecStack<S> {
	stack: Vec<S>,
	logs: [S; instructions::MAX_NO_OF_TOPICS]
}

impl<S> Stack<S> for VecStack<S> {

	fn clear(&mut self) {
		self.stack.clear();
	}

	fn peek(&self, no_from_top: usize) -> &S {
		&self.stack[self.stack.len() - no_from_top - 1]
	}

	fn swap_with_top(&mut self, no_from_top: usize) {
		let len = self.stack.len();
		self.stack.swap(len - no_from_top - 1, len - 1);
	}

	fn has(&self, no_of_elems: usize) -> bool {
		self.stack.len() >= no_of_elems
	}

	fn pop_back(&mut self) -> S {
		let val = self.stack.pop();
		match val {
			Some(x) => x,
			None => panic!("Tried to pop from empty stack.")
		}
	}

	fn pop_n(&mut self, no_of_elems: usize) -> &[S] {
		for i in 0..no_of_elems {
			self.logs[i] = self.pop_back();
		}
		&self.logs[0..no_of_elems]
	}

	fn push(&mut self, elem: S) {
		self.stack.push(elem);
	}

	fn size(&self) -> usize {
		self.stack.len()
	}

	fn peek_top(&self, no_from_top: usize) -> &[S] {
		&self.stack[self.stack.len() - no_from_top .. self.stack.len()]
	}
}

pub struct ShareableStack<S> {
	stack: VecStack<S>,
	bottom: Vec<usize>,
}

impl<S: Default> Default for ShareableStack<S> {
	fn default() -> Self {
		ShareableStack {
			stack: VecStack::default(),
			bottom: vec![0],
		}
	}
}

impl<S> ShareableStack<S> {
	pub fn checkpoint(&mut self) {
		self.bottom.push(self.stack.size());
	}

	pub fn pop_checkpoint(&mut self) {
		assert!(self.bottom.len() > 1);
		self.clear();
		self.bottom.pop();
	}

	fn bottom(&self) -> usize {
		self.bottom[self.bottom.len() - 1]
	}
}

impl<S> Stack<S> for ShareableStack<S> {
	fn clear(&mut self) {
		let bottom = self.bottom();
		self.stack.stack.truncate(bottom);
	}

	fn peek(&self, no_from_top: usize) -> &S {
		assert!(self.has(no_from_top), "peek asked for more items than exist.");
		&self.stack.stack[self.stack.size() - no_from_top - 1]
	}

	fn swap_with_top(&mut self, no_from_top: usize) {
		assert!(self.has(no_from_top), "swap_with_top asked for more items than exist.");
		self.stack.swap_with_top(no_from_top);
	}

	fn has(&self, no_of_elems: usize) -> bool {
		self.stack.size() >= no_of_elems + self.bottom()
	}

	fn pop_back(&mut self) -> S {
		assert!(self.has(1), "Tried to pop from empty stack.");
		self.stack.pop_back()
	}

	fn pop_n(&mut self, no_of_elems: usize) -> &[S] {
		assert!(self.has(no_of_elems), "Tried to pop_n more then there is on stack.");
		self.stack.pop_n(no_of_elems)
	}

	fn push(&mut self, elem: S) {
		self.stack.push(elem);
	}

	fn size(&self) -> usize {
		self.stack.size() - self.bottom()
	}

	fn peek_top(&self, no_from_top: usize) -> &[S] {
		assert!(self.has(no_from_top), "peek_top asked for more items than exist.");
		self.stack.peek_top(no_from_top)
	}
}

#[cfg(test)]
mod tests {
	use super::{ShareableStack, Stack};


	#[test]
	fn should_allow_to_checkpoint_shared_stack() {
		// given
		let mut stack = ShareableStack::default();
		stack.push(10);

		// when
		stack.checkpoint();
		assert_eq!(stack.has(1), false);
		assert_eq!(stack.size(), 0);
		stack.push(5);
		assert_eq!(stack.has(1), true);
		assert_eq!(stack.size(), 1);
		stack.clear();
		assert_eq!(stack.size(), 0);

		// and then
		stack.pop_checkpoint();
		assert_eq!(stack.has(1), true);
		assert_eq!(stack.size(), 1);
		assert_eq!(stack.pop_back(), 10);
		assert_eq!(stack.size(), 0);
	}
}
