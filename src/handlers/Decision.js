import { BusinessKnowledgeModel } from './BusinessKnowledgeModel.js';
import { Expression } from './Expression.js';
import { InputData } from './InputData.js';

export class Decision {
  constructor(moddleDecision, resolver) {
    this.name = moddleDecision.get('name');
    this.logic = new Expression(moddleDecision.get('decisionLogic'));

    this.requiredInputs = [];
    this.requiredDecisions = [];
    this.requiredBKMs = [];

    for (const informationRequirement of moddleDecision.get('informationRequirement')) {
      const requiredInput = informationRequirement.get('requiredInput');
      if (requiredInput) {
        const inputData = resolver.resolve(moddleDecision, requiredInput.get('href'));
        this.requiredInputs.push(new InputData(inputData, resolver));
        continue;
      }

      const requiredDecision = informationRequirement.get('requiredDecision');
      if (requiredDecision) {
        const decision = resolver.resolve(moddleDecision, requiredDecision.get('href'));
        this.requiredDecisions.push(new Decision(decision, resolver));
        continue;
      }
    }

    for (const knowledgeRequirement of moddleDecision.get('knowledgeRequirement')) {
      const requiredBKM = knowledgeRequirement.get('requiredKnowledge');
      if (requiredBKM) {
        const bkm = resolver.resolve(moddleDecision, requiredBKM.get('href'));
        this.requiredBKMs.push(new BusinessKnowledgeModel(bkm, resolver));
      }
    }
  }

  evaluate(context) {
    const ctx = this.createContext(context);

    return this.logic.evaluate(ctx);
  }

  /**
   * Get the name of the decision.
   *
   * @returns {string} The name of the decision.
   */
  getName() {
    return this.name;
  }

  createContext(context) {
    const ctx = { ...context };

    this.requiredDecisions.forEach((decision) => {
      ctx[decision.getName()] = decision.evaluate(context);
    });

    this.requiredBKMs.forEach((bkm) => {
      ctx[bkm.getName()] = bkm.evaluate(context);
    });

    return ctx;
  }
}