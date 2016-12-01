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

import React, { Component, PropTypes } from 'react';
import BigNumber from 'bignumber.js';

import { Input } from '../../../ui/Form';
import GasPriceSelector from './GasPriceSelector';

import styles from './gasAdjustment.css';

export default class GasAdjustment extends Component {
  static propTypes = {
    amount: PropTypes.instanceOf(BigNumber).isRequired,
    amountEstimate: PropTypes.instanceOf(BigNumber).isRequired,
    amountError: PropTypes.string,
    price: PropTypes.instanceOf(BigNumber).isRequired,
    priceDefault: PropTypes.instanceOf(BigNumber).isRequired,
    priceError: PropTypes.string,
    priceHistogram: PropTypes.object.isRequired,
    priceChartHeight: PropTypes.number,
    total: PropTypes.string.isRequired,
    totalError: PropTypes.string,
    onSetAmount: PropTypes.func.isRequired,
    onSetPrice: PropTypes.func.isRequired
  }

  static defaultProps = {
    priceChartHeight: 200
  }

  render () {
    const {
      amount, amountError, amountEstimate,
      price, priceDefault, priceError, priceHistogram, priceChartHeight,
      total, totalError
    } = this.props;

    return (
      <div className={ styles.container }>
        <div className={ styles.columns }>
          <div className={ styles.left }>
            <GasPriceSelector
              gasPriceHistogram={ priceHistogram }
              gasPrice={ price.toString() }
              height={ priceChartHeight }
              onChange={ this.onSetPrice }
            />
          </div>

          <div className={ `${styles.rows} ${styles.right}` }>
            <Input
              label={ `gas amount (estimated: ${amountEstimate.toFormat(0)})` }
              hint='the amount of gas to use for the transaction'
              error={ amountError }
              value={ amount.toString() }
              onChange={ this.onSetAmount } />

            <Input
              label={ `gas price (recommended: ${priceDefault.toFormat(0)})` }
              hint='the price of gas to use for the transaction'
              error={ priceError }
              value={ price.toString() }
              onChange={ this.onSetPrice } />

            <div className={ styles.total }>
              <Input
                readOnly
                label='total transaction amount'
                hint='the total amount of the transaction'
                error={ totalError }
                value={ `${total} ETH` } />
            </div>
          </div>
        </div>

        <div>
          <p>
            You can choose the gas price based on the
            distribution of recent included transactions' gas prices.
            The lower the gas price is, the cheaper the transaction will
            be. The higher the gas price is, the faster it should
            get mined by the network.
          </p>
        </div>
      </div>
    );
  }

  onSetAmount = (event) => {
    this.props.onSetAmount(new BigNumber(event.target.value));
  }

  onSetPrice = (_, value) => {
    this.props.onSetPrice(new BigNumber(value));
  }
}
