// Copyright 2015, 2016 Parity Technologies (UK) Ltd.
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

import { createIdentityImg } from '~/api/util/identity';

let instance;

export default class IdentityIconStore {

  icons = {};

  static get () {
    if (!instance) {
      instance = new IdentityIconStore();
    }

    return instance;
  }

  static getIcon (address, scale) {
    return IdentityIconStore.get()._getIcon(address, scale);
  }

  _getIcon (address, scale) {
    if (!this.icons[address]) {
      this.icons[address] = {};
    }

    const cachedIcon = this.icons[address];
    const cachedScale = cachedIcon[scale];

    if (cachedScale) {
      return cachedScale;
    }

console.warn('Creating identity icon for', address, scale);
    const icon = createIdentityImg(address, scale);
    this.icons[address][scale] = icon;
    return icon;
  }

}
