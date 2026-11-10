import { Engine } from '../../src/engine.js';
import { getFixture } from '../helper.js';

describe('evaluator', function() {

  it('should evaluate a decision', async function() {

    // given
    const engine = new Engine();
    const dmn = getFixture('simple.dmn');
    await engine.load(dmn);

    // when
    const result = await engine.evaluateDecision('Decision');

    // then
    expect(result).to.eql(4);
  });


  it('should evaluate a decision with input', async function() {

    // given
    const engine = new Engine();
    const dmn = getFixture('input.dmn');
    await engine.load(dmn);

    // when
    const result = await engine.evaluateDecision('Decision', { a: 2, b: 2 });

    // then
    expect(result).to.eql(4);
  });


  it('should evaluate a DMN 1.1 decision', async function() {

    // given
    const engine = new Engine();
    await engine.load(getFixture('dmn11.dmn'));

    // then
    expect(await engine.evaluateDecision('Dish', { season: 'Winter' })).to.eql('Roastbeef');
    expect(await engine.evaluateDecision('Dish', { season: 'Summer' })).to.eql('Salad');
  });


  it('should throw on unknown decision', async function() {

    // given
    const engine = new Engine();
    await engine.load(getFixture('simple.dmn'));

    // when
    let error;

    try {
      await engine.evaluateDecision('Unknown');
    } catch (e) {
      error = e;
    }

    // then
    expect(error).to.exist;
    expect(error.message).to.eql('Decision <Unknown> not found');
  });


  describe('decision table', function() {

    let engine;

    beforeEach(async function() {
      engine = new Engine();
      await engine.load(getFixture('decisionTable.dmn'));
    });


    it('should apply UNIQUE hit policy', async function() {

      // then
      expect(await engine.evaluateDecision('Discount', { 'Customer': 'Business', 'Order Size': 5 })).to.eql(0.1);
      expect(await engine.evaluateDecision('Discount', { 'Customer': 'Business', 'Order Size': 10 })).to.eql(0.15);
      expect(await engine.evaluateDecision('Discount', { 'Customer': 'Private', 'Order Size': 10 })).to.eql(0.05);
    });


    it('should use default output if no rule matches', async function() {

      // then
      expect(await engine.evaluateDecision('Discount', { 'Customer': 'Private', 'Order Size': 1 })).to.eql(0);
    });


    it('should reject input not in input values', async function() {

      // when
      const { value, messages } = await engine.evaluate('decision', 'Discount', {
        'Customer': 'Government',
        'Order Size': 1
      });

      // then
      expect(value).to.be.null;
      expect(messages.some(m => m.severity === 'error' && m.message.includes('not one of the allowed values'))).to.be.true;
    });


    it('should apply COLLECT hit policy with SUM aggregation', async function() {

      // then
      expect(await engine.evaluateDecision('Bonus', { 'Order Size': 15 })).to.eql(31);
      expect(await engine.evaluateDecision('Bonus', { 'Order Size': 0 })).to.eql(1);
    });


    it('should apply PRIORITY hit policy', async function() {

      // then
      expect(await engine.evaluateDecision('Rating', { 'Order Size': 12 })).to.eql('high');
      expect(await engine.evaluateDecision('Rating', { 'Order Size': 7 })).to.eql('medium');
      expect(await engine.evaluateDecision('Rating', { 'Order Size': 1 })).to.eql('low');
    });


    it('should apply RULE ORDER hit policy with multiple outputs', async function() {

      // then
      expect(await engine.evaluateDecision('Classification', { 'Order Size': 4 })).to.eql([
        { label: 'positive', weight: 1 },
        { label: 'even', weight: 2 }
      ]);

      expect(await engine.evaluateDecision('Classification', { 'Order Size': 3 })).to.eql([
        { label: 'positive', weight: 1 }
      ]);

      expect(await engine.evaluateDecision('Classification', { 'Order Size': -1 })).to.eql([]);
    });
  });


  describe('decision service', function() {

    let engine;

    beforeEach(async function() {
      engine = new Engine();
      await engine.load(getFixture('decisionService.dmn'));
    });


    it('should evaluate decision service', async function() {

      // when
      const result = await engine.evaluateDecisionService('Calculate', { Amount: 100, Rate: 0.5 });

      // then
      expect(result).to.eql(150);
    });


    it('should evaluate decision service with multiple outputs', async function() {

      // when
      const result = await engine.evaluateDecisionService('Breakdown', { Amount: 100, Rate: 0.5 });

      // then
      expect(result).to.eql({
        Interest: 50,
        Total: 150
      });
    });


    it('should invoke decision service and BKM from decision', async function() {

      // when
      const result = await engine.evaluateDecision('Quote');

      // then
      expect(result).to.eql(220);
    });


    it('should invoke business knowledge model', async function() {

      // then
      expect(await engine.evaluateBusinessKnowledgeModel('Double', [ 21 ])).to.eql(42);
      expect(await engine.evaluateBusinessKnowledgeModel('Double', { x: 4 })).to.eql(8);
    });
  });


  describe('types', function() {

    let engine;

    beforeEach(async function() {
      engine = new Engine();
      await engine.load(getFixture('types.dmn'));
    });


    it('should accept conforming input', async function() {

      // when
      const result = await engine.evaluateDecision('Greeting', {
        Person: { name: 'Ada', age: 36 }
      });

      // then
      expect(result).to.eql('Hello Ada');
    });


    it('should coerce non-conforming input to null', async function() {

      // when
      const { value, messages } = await engine.evaluate('decision', 'Greeting', {
        Person: { name: 'Ada', age: 200 }
      });

      // then
      expect(value).to.be.null;
      expect(messages.some(m => m.severity === 'error' && m.elementName === 'Person')).to.be.true;
    });


    it('should convert to singleton list', async function() {

      // when
      const result = await engine.evaluateDecision('Names', {
        Person: { name: 'Ada', age: 36 }
      });

      // then
      expect(result).to.eql([ 'Ada' ]);
    });


    it('should coerce non-conforming result to null', async function() {

      // when
      const { value, messages } = await engine.evaluate('decision', 'Invalid');

      // then
      expect(value).to.be.null;
      expect(messages).to.have.length(1);
      expect(messages[0]).to.include({
        severity: 'error',
        elementName: 'Invalid'
      });
    });


    it('should check item definitions with instance of', async function() {

      // then
      expect(await engine.evaluateDecision('IsPerson', {
        Person: { name: 'Ada', age: 36 }
      })).to.be.true;
    });
  });


  describe('imports', function() {

    let engine;

    beforeEach(async function() {
      engine = new Engine();

      // models may be loaded in any order
      await engine.load(getFixture('imports/main.dmn'));
      await engine.load(getFixture('imports/lib.dmn'));
    });


    it('should evaluate imported elements, inputs nested by import name', async function() {

      // when
      const result = await engine.evaluateDecision('Welcome', { lib: { Name: 'Ada' } });

      // then
      expect(result).to.eql('HELLO ADA!');
    });


    it('should evaluate imported elements, inputs qualified by namespace', async function() {

      // when
      const result = await engine.evaluateDecision('Welcome', { '{http://jsdmn/lib}Name': 'Ada' });

      // then
      expect(result).to.eql('HELLO ADA!');
    });


    it('should look up decision by namespace', async function() {

      // when
      const result = await engine.evaluateDecision('Salutation', { Name: 'Bob' }, {
        namespace: 'http://jsdmn/lib'
      });

      // then
      expect(result).to.eql('Hello Bob');
    });
  });
});
