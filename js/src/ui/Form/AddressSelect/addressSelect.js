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

import React, { Component, PropTypes } from 'react';
import ReactDOM from 'react-dom';
import { connect } from 'react-redux';
import keycode, { codes } from 'keycode';
import { FormattedMessage } from 'react-intl';
import { observer } from 'mobx-react';

import TextFieldUnderline from 'material-ui/TextField/TextFieldUnderline';

import AccountCard from '~/ui/AccountCard';
import InputAddress from '~/ui/Form/InputAddress';
import Portal from '~/ui/Portal';

import AddressSelectStore from './addressSelectStore';
import styles from './addressSelect.css';

const BOTTOM_BORDER_STYLE = { borderBottom: 'solid 3px' };

@observer
class AddressSelect extends Component {
  static contextTypes = {
    api: PropTypes.object.isRequired,
    muiTheme: PropTypes.object.isRequired
  };

  static propTypes = {
    // Required props
    onChange: PropTypes.func.isRequired,

    // Redux props
    accountsInfo: PropTypes.object,
    accounts: PropTypes.object,
    balances: PropTypes.object,
    contacts: PropTypes.object,
    contracts: PropTypes.object,
    tokens: PropTypes.object,
    wallets: PropTypes.object,

    // Optional props
    allowInput: PropTypes.bool,
    disabled: PropTypes.bool,
    error: PropTypes.string,
    hint: PropTypes.string,
    label: PropTypes.string,
    value: PropTypes.string
  };

  static defaultProps = {
    value: ''
  };

  state = {
    expanded: false,
    focused: false,
    focusedCat: null,
    focusedItem: null,
    inputFocused: false,
    inputValue: ''
  };

  store = new AddressSelectStore(this.context.api);

  componentWillMount () {
    this.setValues();
  }

  componentWillReceiveProps (nextProps) {
    if (this.store.values && this.store.values.length > 0) {
      return;
    }

    this.setValues(nextProps);
  }

  setValues (props = this.props) {
    this.store.setValues(props);
  }

  render () {
    const input = this.renderInput();
    const content = this.renderContent();

    const classes = [ styles.main ];

    return (
      <div
        className={ classes.join(' ') }
        onBlur={ this.handleMainBlur }
        onClick={ this.handleFocus }
        onFocus={ this.handleMainFocus }
        onKeyDown={ this.handleInputAddresKeydown }
        ref='inputAddress'
        tabIndex={ 0 }
      >
        { input }
        { content }
      </div>
    );
  }

  renderInput () {
    const { focused } = this.state;
    const { accountsInfo, disabled, error, hint, label, value } = this.props;

    const input = (
      <InputAddress
        accountsInfo={ accountsInfo }
        allowCopy={ false }
        disabled
        error={ error }
        hint={ hint }
        focused={ focused }
        label={ label }
        tabIndex={ -1 }
        text
        value={ value }
      />
    );

    if (disabled) {
      return input;
    }

    return (
      <div className={ styles.inputAddress }>
        { input }
      </div>
    );
  }

  renderContent () {
    const { muiTheme } = this.context;
    const { hint, disabled, label } = this.props;
    const { expanded, inputFocused } = this.state;

    if (disabled) {
      return null;
    }

    const id = 'addressSelect_' + Math.round(Math.random() * 100).toString();

    return (
      <Portal
        className={ styles.inputContainer }
        onClose={ this.handleClose }
        onKeyDown={ this.handleKeyDown }
        open={ expanded }
      >
        <label className={ styles.label } htmlFor={ id }>
          { label }
        </label>
        <input
          id={ id }
          className={ styles.input }
          placeholder={ hint }

          onBlur={ this.handleInputBlur }
          onFocus={ this.handleInputFocus }
          onChange={ this.handleChange }

          ref={ this.setInputRef }
        />

        <div className={ styles.underline }>
          <TextFieldUnderline
            focus={ inputFocused }
            focusStyle={ BOTTOM_BORDER_STYLE }
            muiTheme={ muiTheme }
            style={ BOTTOM_BORDER_STYLE }
          />
        </div>

        { this.renderCurrentInput() }
        { this.renderRegsitryValues() }
        { this.renderAccounts() }
      </Portal>
    );
  }

  renderCurrentInput () {
    if (!this.props.allowInput) {
      return null;
    }

    const { inputValue } = this.state;

    if (inputValue.length === 0 || !/^(0x)?[a-f0-9]*$/i.test(inputValue)) {
      return null;
    }

    return (
      <div>
        { this.renderAccountCard({ address: inputValue }) }
      </div>
    );
  }

  renderRegsitryValues () {
    const { regsitryValues } = this.store;

    if (regsitryValues.length === 0) {
      return null;
    }

    const accounts = regsitryValues
      .map((regsitryValue) => {
        const { address, value } = regsitryValue;
        const account = { address, name: value, index: address };

        return this.renderAccountCard(account);
      });

    return (
      <div>
        { accounts }
      </div>
    );
  }

  renderAccounts () {
    const { values } = this.store;

    if (values.length === 0) {
      return (
        <div className={ styles.categories }>
          <div className={ styles.empty }>
            <FormattedMessage
              id='addressSelect.noAccount'
              defaultMessage='No account matches this query...'
            />
          </div>
        </div>
      );
    }

    const categories = values.map((category) => {
      return this.renderCategory(category.label, category.values);
    });

    return (
      <div className={ styles.categories }>
        { categories }
      </div>
    );
  }

  renderCategory (name, values = []) {
    if (values.length === 0) {
      return null;
    }

    const cards = values
      .map((account) => this.renderAccountCard(account));

    return (
      <div className={ styles.category } key={ name }>
        <div className={ styles.title }>{ name }</div>
        <div className={ styles.cards }>
          <div>{ cards }</div>
        </div>
      </div>
    );
  }

  renderAccountCard (_account) {
    const { balances, accountsInfo } = this.props;
    const { address, index = null } = _account;

    const balance = balances[address];
    const account = {
      ...accountsInfo[address],
      ..._account
    };

    return (
      <AccountCard
        account={ account }
        balance={ balance }
        key={ `account_${index}` }
        onClick={ this.handleClick }
        onFocus={ this.focusItem }
        ref={ `account_${index}` }
      />
    );
  }

  setInputRef = (refId) => {
    this.inputRef = refId;
  }

  validateCustomInput = () => {
    const { allowInput } = this.props;
    const { inputValue } = this.state;
    const { values } = this.store;

    // If input is HEX and allowInput === true, send it
    if (allowInput && inputValue && /^(0x)?([0-9a-f])+$/i.test(inputValue)) {
      return this.handleClick(inputValue);
    }

    // If only one value, select it
    if (values.length === 1 && values[0].values.length === 1) {
      const value = values[0].values[0];
      return this.handleClick(value.address);
    }
  }

  handleInputAddresKeydown = (event) => {
    const code = keycode(event);

    // Simulate click on input address if enter is pressed
    if (code === 'enter') {
      return this.handleDOMAction('inputAddress', 'click');
    }
  }

  handleKeyDown = (event) => {
    const codeName = keycode(event);

    if (event.ctrlKey) {
      return event;
    }

    switch (codeName) {
      case 'enter':
        const index = this.state.focusedItem;
        if (!index) {
          return this.validateCustomInput();
        }

        return this.handleDOMAction(`account_${index}`, 'click');

      case 'left':
      case 'right':
      case 'up':
      case 'down':
        event.preventDefault();
        return this.handleNavigation(codeName);

      default:
        const code = codes[codeName];

        // @see https://github.com/timoxley/keycode/blob/master/index.js
        // lower case chars
        if (code >= (97 - 32) && code <= (122 - 32)) {
          return this.handleDOMAction(this.inputRef, 'focus');
        }

        // numbers
        if (code >= 48 && code <= 57) {
          return this.handleDOMAction(this.inputRef, 'focus');
        }

        return event;
    }
  }

  handleDOMAction = (ref, method) => {
    const refItem = typeof ref === 'string' ? this.refs[ref] : ref;
    const element = ReactDOM.findDOMNode(refItem);

    if (!element || typeof element[method] !== 'function') {
      console.warn('could not find', ref, 'or method', method);
      return;
    }

    return element[method]();
  }

  focusItem = (index) => {
    this.setState({ focusedItem: index });
    return this.handleDOMAction(`account_${index}`, 'focus');
  }

  handleNavigation = (direction) => {
    const { focusedItem, focusedCat } = this.state;
    const { values } = this.store;

    // Don't do anything if no values
    if (values.length === 0) {
      return;
    }

    // Focus on the first element if none selected yet if going down
    if (!focusedItem) {
      if (direction !== 'down') {
        return;
      }

      const nextValues = values[focusedCat || 0];
      const nextFocus = nextValues ? nextValues.values[0] : null;
      return this.focusItem(nextFocus && nextFocus.index || 1);
    }

    // Find the previous focused category
    const prevCategoryIndex = values.findIndex((category) => {
      return category.values.find((value) => value.index === focusedItem);
    });
    const prevFocusIndex = values[prevCategoryIndex].values.findIndex((a) => a.index === focusedItem);

    let nextCategory = prevCategoryIndex;
    let nextFocusIndex;

    // If down: increase index if possible
    if (direction === 'down') {
      const prevN = values[prevCategoryIndex].values.length;
      nextFocusIndex = Math.min(prevFocusIndex + 1, prevN - 1);
    }

    // If up: decrease index if possible
    if (direction === 'up') {
      // Focus on search if at the top
      if (prevFocusIndex === 0) {
        return this.handleDOMAction(this.inputRef, 'focus');
      }

      nextFocusIndex = prevFocusIndex - 1;
    }

    // If right: next category
    if (direction === 'right') {
      nextCategory = Math.min(prevCategoryIndex + 1, values.length - 1);
    }

    // If right: previous category
    if (direction === 'left') {
      nextCategory = Math.max(prevCategoryIndex - 1, 0);
    }

    // If left or right: try to keep the horizontal index
    if (direction === 'left' || direction === 'right') {
      this.setState({ focusedCat: nextCategory });
      nextFocusIndex = Math.min(prevFocusIndex, values[nextCategory].values.length - 1);
    }

    const nextFocus = values[nextCategory].values[nextFocusIndex].index;
    return this.focusItem(nextFocus);
  }

  handleClick = (address) => {
    // Don't do anything if it's only text-selection
    if (window.getSelection && window.getSelection().type === 'Range') {
      return;
    }

    this.props.onChange(null, address);
    this.handleClose();
  }

  handleMainBlur = () => {
    if (window.document.hasFocus() && !this.state.expanded) {
      this.closing = false;
      this.setState({ focused: false });
    }
  }

  handleMainFocus = () => {
    if (this.state.focused) {
      return;
    }

    this.setState({ focused: true }, () => {
      if (this.closing) {
        this.closing = false;
        return;
      }

      this.handleFocus();
    });
  }

  handleFocus = () => {
    this.setState({ expanded: true, focusedItem: null, focusedCat: null }, () => {
      window.setTimeout(() => {
        this.handleDOMAction(this.inputRef, 'focus');
      });
    });
  }

  handleClose = () => {
    this.closing = true;

    if (this.refs.inputAddress) {
      this.handleDOMAction('inputAddress', 'focus');
    }

    this.setState({ expanded: false });
  }

  handleInputBlur = () => {
    this.setState({ inputFocused: false });
  }

  handleInputFocus = () => {
    this.setState({ focusedItem: null, inputFocused: true });
  }

  handleChange = (event = { target: {} }) => {
    const { value = '' } = event.target;

    this.store.handleChange(value);

    this.setState({
      focusedItem: null,
      inputValue: value
    });
  }
}

function mapStateToProps (state) {
  const { accountsInfo } = state.personal;
  const { balances } = state.balances;

  return {
    accountsInfo,
    balances
  };
}

export default connect(
  mapStateToProps
)(AddressSelect);
