import { BusinessKnowledgeModel } from './handlers/BusinessKnowledgeModel.js';
import { Decision } from './handlers/Decision.js';
import { DecisionService } from './handlers/DecisionService.js';
import { InputData } from './handlers/InputData.js';

import {
  ANY,
  ConstrainedType,
  ContextType,
  FunctionType,
  ListType,
  NamedType,
  coerce,
  getBuiltinType,
  parseTypeRef
} from './types.js';

import { getFeelType } from './feel.js';

const DRG_HANDLERS = {
  'dmn:Decision': Decision,
  'dmn:InputData': InputData,
  'dmn:BusinessKnowledgeModel': BusinessKnowledgeModel,
  'dmn:DecisionService': DecisionService
};

const DMN_IMPORT_TYPES = [
  'http://www.omg.org/spec/DMN/20151101/dmn.xsd',
  'http://www.omg.org/spec/DMN/20180521/MODEL/',
  'https://www.omg.org/spec/DMN/20191111/MODEL/',
  'https://www.omg.org/spec/DMN/20211108/MODEL/',
  'https://www.omg.org/spec/DMN/20230324/MODEL/'
];


/**
 * A loaded DMN model (`<definitions>`).
 */
export class Model {

  /**
   * @param {object} definitions moddle `dmn:Definitions`
   * @param {object} registry
   * @param {(namespace: string) => Model|null} registry.getModel
   */
  constructor(definitions, registry) {
    this.definitions = definitions;
    this.registry = registry;

    this.id = definitions.get('id');
    this.name = definitions.get('name');
    this.namespace = definitions.get('namespace');

    this.imports = definitions.get('import')
      .filter(i => !i.get('importType') || DMN_IMPORT_TYPES.includes(i.get('importType')))
      .map(i => ({
        name: i.get('name') || '',
        namespace: i.get('namespace')
      }));

    /**
     * @type {Map<string, object>}
     */
    this.itemDefinitions = new Map();

    for (const itemDefinition of definitions.get('itemDefinition')) {
      this.itemDefinitions.set(itemDefinition.get('name'), itemDefinition);
    }

    this._types = new Map();

    this.elements = [];

    /**
     * @type {Map<string, object>}
     */
    this.elementsById = new Map();

    for (const element of definitions.get('drgElement')) {
      const Handler = DRG_HANDLERS[element.$type];

      if (!Handler) {
        continue;
      }

      const handler = new Handler(element, this);

      this.elements.push(handler);
      this.elementsById.set(element.get('id'), handler);
    }
  }

  /**
   * @param {string} name
   * @param {Function} [Type] handler class
   */
  getElement(name, Type) {
    return this.elements.find(e => e.name === name && (!Type || e instanceof Type)) || null;
  }

  getDecision(name) {
    return this.getElement(name, Decision);
  }

  getDecisionService(name) {
    return this.getElement(name, DecisionService);
  }

  getBusinessKnowledgeModel(name) {
    return this.getElement(name, BusinessKnowledgeModel);
  }

  getInputData(name) {
    return this.getElement(name, InputData);
  }

  /**
   * Resolve a DMN element reference (`#id` or `namespace#id`).
   *
   * @param {string} href
   *
   * @return {object|null}
   */
  resolveReference(href) {

    if (!href) {
      return null;
    }

    const separatorIdx = href.lastIndexOf('#');

    const namespace = separatorIdx > 0 ? href.slice(0, separatorIdx) : null;
    const id = href.slice(separatorIdx + 1);

    const model = !namespace || namespace === this.namespace
      ? this
      : this.registry.getModel(namespace);

    return model?.elementsById.get(id) || null;
  }

  /**
   * Names under which elements of the given model are visible in this
   * model; an empty string denotes an unnamed import.
   *
   * @param {Model} model
   *
   * @return {string[]}
   */
  getImportNames(model) {
    return this.imports
      .filter(i => i.namespace === model.namespace)
      .map(i => i.name);
  }

  getImportedModel(importName) {
    const i = this.imports.find(i => i.name === importName);

    return i ? this.registry.getModel(i.namespace) : null;
  }

  /**
   * Make an element's value visible in the given scope, under its name
   * or qualified with the name of the import it originates from.
   *
   * @param {object} scope
   * @param {object} element
   * @param {any} value
   */
  assign(scope, element, value) {

    if (element.model === this) {
      scope[element.name] = value;

      return;
    }

    const importNames = this.getImportNames(element.model);

    if (!importNames.length) {
      scope[element.name] = value;

      return;
    }

    for (const importName of importNames) {

      if (!importName) {
        scope[element.name] = value;
      } else {
        scope[importName] = {
          ...(isPlainObject(scope[importName]) ? scope[importName] : {}),
          [element.name]: value
        };
      }
    }
  }


  // types ///////////////

  /**
   * Resolve a type reference in the context of this model.
   *
   * @param {string} [typeRef]
   *
   * @return {import('./types.js').Type}
   */
  resolveType(typeRef) {

    if (!typeRef || !typeRef.trim()) {
      return ANY;
    }

    if (this._types.has(typeRef)) {
      return this._types.get(typeRef);
    }

    let type;

    try {
      type = this.resolveTypeName(typeRef) || parseTypeRef(typeRef, name => this.resolveTypeName(name));
    } catch (error) {
      type = ANY;
    }

    this._types.set(typeRef, type);

    return type;
  }

  /**
   * @param {string} name
   *
   * @return {import('./types.js').Type|null}
   */
  resolveTypeName(name) {

    const builtin = getBuiltinType(name);

    if (builtin) {
      return builtin;
    }

    const itemDefinition = this.itemDefinitions.get(name);

    if (itemDefinition) {
      return this.getItemDefinitionType(itemDefinition);
    }

    // imported type, e.g. `myImport.tPerson`
    const separatorIdx = name.indexOf('.');

    if (separatorIdx > 0) {
      const importedModel = this.getImportedModel(name.slice(0, separatorIdx));

      if (importedModel) {
        return importedModel.resolveTypeName(name.slice(separatorIdx + 1));
      }
    }

    // unnamed imports
    for (const i of this.imports.filter(i => !i.name)) {
      const importedModel = this.registry.getModel(i.namespace);

      const type = importedModel && importedModel !== this && importedModel.itemDefinitions.has(name)
        ? importedModel.resolveTypeName(name)
        : null;

      if (type) {
        return type;
      }
    }

    return null;
  }

  getItemDefinitionType(itemDefinition) {

    const key = `#item:${ itemDefinition.get('id') || itemDefinition.get('name') }`;

    let type = this._types.get(key);

    if (!type) {
      type = this.createItemDefinitionType(itemDefinition);

      this._types.set(key, type);
    }

    return type;
  }

  createItemDefinitionType(itemDefinition) {

    return new NamedType(itemDefinition.get('name'), () => {

      const functionItem = itemDefinition.get('functionItem');
      const components = itemDefinition.get('itemComponent');

      let type;

      if (functionItem) {
        type = new FunctionType(
          functionItem.get('parameters').map(p => this.resolveType(p.get('typeRef'))),
          this.resolveType(functionItem.get('outputTypeRef'))
        );
      } else if (components.length) {
        type = new ContextType(
          components.map(component => [ component.get('name'), this.getItemDefinitionType(component) ]),
          itemDefinition.get('name')
        );
      } else {
        type = this.resolveType(itemDefinition.get('typeRef'));
      }

      const allowedValues = itemDefinition.get('allowedValues')?.get('text');

      if (allowedValues && allowedValues.trim()) {
        type = new ConstrainedType(type, allowedValues);
      }

      if (itemDefinition.get('isCollection')) {
        type = new ListType(type);
      }

      const typeConstraint = itemDefinition.get('typeConstraint')?.get('text');

      if (typeConstraint && typeConstraint.trim()) {
        type = new ConstrainedType(type, typeConstraint);
      }

      return type;
    });
  }

  /**
   * Create the root scope for evaluating expressions of this model.
   */
  createScope() {
    return {};
  }

  /**
   * Coerce a value to a type, reporting non-conformance.
   *
   * @param {any} value
   * @param {import('./types.js').Type} type
   * @param {import('./evaluation.js').Evaluation} evaluation
   * @param {object} element
   * @param {string} [what]
   *
   * @return {any}
   */
  coerce(value, type, evaluation, element, what = 'value') {

    const result = coerce(value, type);

    if (result.error) {
      evaluation?.error(`${ what } of <${ element?.name || element?.id || '?' }> is invalid: ${ result.error }`, element);
    }

    return result.value;
  }

  toString() {
    return `Model <${ this.name || this.namespace }>`;
  }
}


function isPlainObject(value) {
  return getFeelType(value) === 'context';
}
