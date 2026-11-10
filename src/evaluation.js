/**
 * @typedef {object} Message
 * @property {'error'|'warning'} severity
 * @property {string} message
 * @property {string} [type]
 * @property {string} [elementId] id of the element (e.g. the expression) reporting the message
 * @property {string} [elementName] name of the DRG element (decision, BKM, ...) the message originates from
 */

/**
 * State of a single evaluation request: provided inputs, the results of
 * decisions evaluated so far and the messages reported along the way.
 */
export class Evaluation {

  /**
   * @param {object} options
   * @param {(inputData: import('./handlers/InputData.js').InputData) => any} options.getInput
   *        returns the raw value provided for an input data element (or `undefined`)
   * @param {Message[]} [options.messages] shared message log
   * @param {object} [options.variables] free variables visible to decisions, i.e. provided
   *        values that do not correspond to a modeled input data element
   */
  constructor({ getInput, messages = [], variables = {} }) {
    this.getInput = getInput;
    this.messages = messages;
    this.variables = variables;

    /**
     * @type {Map<object, any>}
     */
    this.results = new Map();

    /**
     * @type {Set<object>}
     */
    this.pending = new Set();

    /**
     * DRG elements currently being evaluated.
     *
     * @type {object[]}
     */
    this.stack = [];
  }

  /**
   * Create an evaluation sharing the message log of this one.
   */
  fork(getInput) {
    return new Evaluation({ getInput, messages: this.messages, variables: this.variables });
  }

  /**
   * Evaluate a DRG element (decision or input data) at most once.
   *
   * @param {object} element
   * @param {() => any} evaluate
   */
  resolve(element, evaluate) {

    if (this.results.has(element)) {
      return this.results.get(element);
    }

    if (this.pending.has(element)) {
      this.error(`cyclic dependency detected on <${ element.name }>`, element);

      return null;
    }

    this.pending.add(element);

    try {
      const value = this.within(element, evaluate);

      this.results.set(element, value);

      return value;
    } finally {
      this.pending.delete(element);
    }
  }

  /**
   * Evaluate in the context of the given DRG element, attributing
   * messages to it.
   *
   * @param {object} element
   * @param {() => any} evaluate
   */
  within(element, evaluate) {

    this.stack.push(element);

    try {
      return evaluate();
    } finally {
      this.stack.pop();
    }
  }

  /**
   * Provide the result of a DRG element upfront (e.g. a decision service
   * input decision).
   */
  provide(element, value) {
    this.results.set(element, value);
  }

  error(message, element, type) {
    this.addMessage('error', message, element, type);
  }

  warn(message, element, type) {
    this.addMessage('warning', message, element, type);
  }

  addMessage(severity, message, element, type) {

    const owner = element?.name ? element : this.stack[this.stack.length - 1];

    this.messages.push({
      severity,
      message,
      type,
      elementId: element?.id ?? owner?.id,
      elementName: element?.name ?? owner?.name
    });
  }

  hasErrors() {
    return this.messages.some(message => message.severity === 'error');
  }
}
