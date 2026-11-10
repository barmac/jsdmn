import { Model } from './model.js';
import { parseDmn } from './parser.js';

/**
 * Loads DMN models and keeps track of them, so that models can import
 * each other.
 */
export class Loader {

  constructor() {

    /**
     * @type {Model[]}
     */
    this.models = [];
  }

  /**
   * Load a DMN model from XML.
   *
   * @param {string} xml
   *
   * @return {Promise<Model>}
   */
  async load(xml) {
    const { definitions, version } = await parseDmn(xml);

    const model = new Model(definitions, this, { version });

    // a model re-loaded under the same namespace replaces the old one
    this.models = this.models.filter(m => m.namespace !== model.namespace);

    this.models.push(model);

    return model;
  }

  /**
   * @param {string} namespace
   *
   * @return {Model|null}
   */
  getModel(namespace) {
    return this.models.find(model => model.namespace === namespace) || null;
  }

  /**
   * Find a model by namespace, by name or fall back to the first
   * model that contains a matching element.
   *
   * @param {(model: Model) => any} predicate
   * @param {string} [namespace]
   *
   * @return {any}
   */
  find(predicate, namespace) {

    const models = namespace
      ? this.models.filter(m => m.namespace === namespace || m.name === namespace)
      : this.models;

    for (const model of models) {
      const result = predicate(model);

      if (result) {
        return result;
      }
    }

    return null;
  }

  getDecision(name, namespace) {
    return this.find(model => model.getDecision(name), namespace);
  }

  getDecisionService(name, namespace) {
    return this.find(model => model.getDecisionService(name), namespace);
  }

  getBusinessKnowledgeModel(name, namespace) {
    return this.find(model => model.getBusinessKnowledgeModel(name), namespace);
  }
}
