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
import { Link } from 'react-router';
import { isEqual, pick } from 'lodash';

import { Container, ContainerTitle, Tags } from '~/ui';

import styles from './summary.css';

export default class Summary extends Component {
  static contextTypes = {
    api: React.PropTypes.object
  };

  static propTypes = {
    app: PropTypes.object.isRequired,
    className: PropTypes.string
  };

  static defaultProps = {
    className: ''
  };

  shouldComponentUpdate (nextProps) {
    const keys = [ 'type', 'id', 'name', 'description', 'author', 'version', 'iconUrl', 'image' ];

    return !isEqual(pick(nextProps, keys), pick(this.props, keys));
  }

  render () {
    const { app, className } = this.props;
    const { type, id, name, description, author, version } = app;

    return (
      <div className={ className }>
        <Container className={ styles.container }>
          { this.renderImage(app) }
          <Tags tags={ [type] } />
          <div className={ styles.description }>
            <ContainerTitle
              className={ styles.title }
              title={
                <Link to={ `/app/${id}` }>
                  { name }
                </Link>
              }
              byline={ description }
            />
            <div className={ styles.author }>
              { author }, v{ version }
            </div>
          </div>
        </Container>
      </div>
    );
  }

  renderImage (app) {
    const { dappsUrl } = this.context.api;
    const { type, id, iconUrl, image } = app;

    return type === 'local'
      ? (
        <img src={ `${dappsUrl}/${id}/${iconUrl}` } className={ styles.image } />
      )
      : (
        <img src={ `${dappsUrl}${image}` } className={ styles.image } />
      );
  }
}
