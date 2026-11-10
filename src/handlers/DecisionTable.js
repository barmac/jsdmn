import {
  evaluateExpression,
  evaluateUnaryTests,
  feelEquals,
  feelLessThan,
  splitUnaryTests,
  describe
} from '../feel.js';

const HIT_POLICY_ALIASES = {
  'U': 'UNIQUE',
  'A': 'ANY',
  'P': 'PRIORITY',
  'F': 'FIRST',
  'C': 'COLLECT',
  'R': 'RULE ORDER',
  'O': 'OUTPUT ORDER'
};

/**
 * A decision table.
 */
export class DecisionTable {

  /**
   * @param {object} element moddle `dmn:DecisionTable`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.model = model;

    const hitPolicy = element.get('hitPolicy') || 'UNIQUE';

    this.hitPolicy = HIT_POLICY_ALIASES[hitPolicy] || hitPolicy;
    this.aggregation = element.get('aggregation') || null;

    this.inputs = element.get('input').map(input => ({
      id: input.get('id'),
      name: input.get('label') || input.get('inputExpression')?.get('text'),
      expression: input.get('inputExpression')?.get('text') || '',
      typeRef: input.get('inputExpression')?.get('typeRef'),
      allowedValues: input.get('inputValues')?.get('text') || null
    }));

    this.outputs = element.get('output').map(output => {
      const allowedValues = output.get('outputValues')?.get('text') || null;

      return {
        id: output.get('id'),
        name: output.get('name'),
        typeRef: output.get('typeRef'),
        allowedValues,
        priorities: allowedValues ? splitUnaryTests(allowedValues) : [],
        defaultValue: output.get('defaultOutputEntry')?.get('text') || null
      };
    });

    this.rules = element.get('rule').map((rule, idx) => ({
      id: rule.get('id'),
      index: idx,
      inputEntries: rule.get('inputEntry').map(entry => entry.get('text')),
      outputEntries: rule.get('outputEntry').map(entry => entry.get('text'))
    }));
  }

  evaluate(scope, evaluation) {

    // evaluate inputs
    const inputValues = [];

    for (const input of this.inputs) {
      const value = this.model.coerce(
        evaluateExpression(input.expression, scope, evaluation, this, this.model),
        this.model.resolveType(input.typeRef),
        evaluation,
        input,
        'input expression'
      );

      if (
        input.allowedValues &&
        value !== null &&
        evaluateUnaryTests(input.allowedValues, value, scope, evaluation, this, this.model) !== true
      ) {
        evaluation.error(`input value <${ describe(value) }> of <${ input.name }> is not one of the allowed values <${ input.allowedValues }>`, this);

        return null;
      }

      inputValues.push(value);
    }

    // match rules
    const matches = this.rules.filter(rule => {
      return rule.inputEntries.every((entry, idx) => {
        return evaluateUnaryTests(entry, inputValues[idx], scope, evaluation, this, this.model) === true;
      });
    });

    if (!matches.length) {
      return this.getDefaultResult(scope, evaluation);
    }

    // evaluate outputs of matched rules
    const results = [];

    for (const rule of matches) {
      const outputs = [];

      for (const [ idx, output ] of this.outputs.entries()) {
        const value = this.model.coerce(
          evaluateExpression(rule.outputEntries[idx], scope, evaluation, this, this.model),
          this.model.resolveType(output.typeRef),
          evaluation,
          output,
          'output'
        );

        if (
          output.allowedValues &&
          value !== null &&
          evaluateUnaryTests(output.allowedValues, value, scope, evaluation, this, this.model) !== true
        ) {
          evaluation.error(`output value <${ describe(value) }> of <${ output.name || 'output' }> is not one of the allowed values <${ output.allowedValues }>`, this);

          return null;
        }

        outputs.push(value);
      }

      results.push({ rule, outputs });
    }

    return this.applyHitPolicy(results, scope, evaluation);
  }

  applyHitPolicy(results, scope, evaluation) {

    const toValue = (result) => this.toValue(result.outputs);

    switch (this.hitPolicy) {

    case 'UNIQUE':
      if (results.length > 1) {
        evaluation.error(`multiple rules (${ results.map(r => r.rule.index + 1).join(', ') }) matched for UNIQUE hit policy`, this);

        return null;
      }

      return toValue(results[0]);

    case 'ANY': {
      const [ first, ...rest ] = results;

      const conflicting = rest.find(result => !outputsEqual(first.outputs, result.outputs));

      if (conflicting) {
        evaluation.error(`rules ${ first.rule.index + 1 } and ${ conflicting.rule.index + 1 } produce different outputs for ANY hit policy`, this);

        return null;
      }

      return toValue(first);
    }

    case 'FIRST':
      return toValue(results[0]);

    case 'PRIORITY':
      return toValue(this.sortByPriority(results, scope, evaluation)[0]);

    case 'RULE ORDER':
      return results.map(toValue);

    case 'OUTPUT ORDER':
      return this.sortByPriority(results, scope, evaluation).map(toValue);

    case 'COLLECT':
      return this.aggregate(results.map(toValue), evaluation);

    default:
      evaluation.error(`unsupported hit policy <${ this.hitPolicy }>`, this);

      return null;
    }
  }

  aggregate(values, evaluation) {

    const { aggregation } = this;

    if (!aggregation) {
      return values;
    }

    if (this.outputs.length > 1) {
      evaluation.error(`aggregation <${ aggregation }> requires a single output`, this);

      return null;
    }

    const nonNull = values.filter(value => value !== null);

    switch (aggregation) {

    case 'COUNT':
      return nonNull.length;

    case 'SUM':
      if (!nonNull.length) {
        return null;
      }

      if (!nonNull.every(value => typeof value === 'number')) {
        evaluation.error('SUM aggregation requires numeric outputs', this);

        return null;
      }

      return nonNull.reduce((sum, value) => sum + value, 0);

    case 'MIN':
    case 'MAX': {

      if (!nonNull.length) {
        return null;
      }

      let result = nonNull[0];

      for (const value of nonNull.slice(1)) {
        const less = aggregation === 'MIN' ? feelLessThan(value, result) : feelLessThan(result, value);

        if (less === null) {
          evaluation.error(`${ aggregation } aggregation requires comparable outputs`, this);

          return null;
        }

        if (less) {
          result = value;
        }
      }

      return result;
    }

    default:
      evaluation.error(`unsupported aggregation <${ aggregation }>`, this);

      return null;
    }
  }

  /**
   * Sort results by output priority (as given by the output values),
   * keeping rule order for results of equal priority.
   */
  sortByPriority(results, scope, evaluation) {

    const priorities = new Map(results.map(result => [
      result,
      this.outputs.map((output, idx) => this.getPriority(output, result.outputs[idx], scope, evaluation))
    ]));

    return [ ...results ].sort((a, b) => {
      const pa = priorities.get(a);
      const pb = priorities.get(b);

      for (let idx = 0; idx < pa.length; idx++) {
        if (pa[idx] !== pb[idx]) {
          return pa[idx] - pb[idx];
        }
      }

      return a.rule.index - b.rule.index;
    });
  }

  getPriority(output, value, scope, evaluation) {

    const idx = output.priorities.findIndex(test => {
      return evaluateUnaryTests(test, value, scope, evaluation, this, this.model) === true;
    });

    return idx === -1 ? output.priorities.length : idx;
  }

  getDefaultResult(scope, evaluation) {

    if (!this.outputs.some(output => output.defaultValue !== null)) {

      if (this.hitPolicy === 'COLLECT' && this.aggregation === 'COUNT') {
        return 0;
      }

      if ([ 'COLLECT', 'RULE ORDER', 'OUTPUT ORDER' ].includes(this.hitPolicy) && !this.aggregation) {
        return [];
      }

      return null;
    }

    return this.toValue(this.outputs.map(output => {
      return output.defaultValue !== null
        ? evaluateExpression(output.defaultValue, scope, evaluation, this, this.model)
        : null;
    }));
  }

  /**
   * A single output yields its value, multiple outputs a context.
   */
  toValue(outputs) {

    if (this.outputs.length === 1) {
      return outputs[0];
    }

    return Object.fromEntries(this.outputs.map((output, idx) => [ output.name, outputs[idx] ]));
  }
}

function outputsEqual(a, b) {
  return a.length === b.length && a.every((value, idx) => {
    return (value === null && b[idx] === null) || feelEquals(value, b[idx]) === true;
  });
}
