import { createExpression } from './Expression.js';

/**
 * A boxed relation; evaluates to a list of contexts.
 */
export class Relation {

  /**
   * @param {object} element moddle `dmn:Relation`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.model = model;

    this.columns = element.get('column').map(column => ({
      name: column.get('name'),
      typeRef: column.get('typeRef')
    }));

    this.rows = element.get('row').map(row => {
      return row.get('elements').map(e => createExpression(e, model));
    });
  }

  evaluate(scope, evaluation) {

    return this.rows.map(row => {

      if (row.length !== this.columns.length) {
        evaluation.error(`relation row has ${ row.length } cells, expected ${ this.columns.length }`, this);

        return null;
      }

      return Object.fromEntries(this.columns.map((column, idx) => {
        const value = row[idx].evaluate(scope, evaluation);

        return [
          column.name,
          this.model.coerce(value, this.model.resolveType(column.typeRef), evaluation, column, 'relation cell')
        ];
      }));
    });
  }
}
