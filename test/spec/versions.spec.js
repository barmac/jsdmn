import { Engine } from '../../src/engine.js';
import { getDefinitionsNamespace } from '../../src/parser.js';
import { getFixture } from '../helper.js';

const DMN13 = 'https://www.omg.org/spec/DMN/20191111/MODEL/';
const DMN14 = 'https://www.omg.org/spec/DMN/20211108/MODEL/';
const DMN15 = 'https://www.omg.org/spec/DMN/20230324/MODEL/';


describe('DMN versions', function() {

  describe('DMN 1.3', function() {

    let engine, model;

    beforeEach(async function() {
      engine = new Engine();
      model = await engine.load(getFixture('versions/dmn13.dmn'));
    });


    it('should detect version', function() {
      expect(model.version).to.eql('1.3');
    });


    it('should evaluate decision table, BKM, invocation and context', async function() {

      // then
      expect(await engine.evaluateDecision('Price', { Age: 10 })).to.eql(50);
      expect(await engine.evaluateDecision('Price', { Age: 40 })).to.eql(100);
      expect(await engine.evaluateDecision('Price', { Age: 70 })).to.eql(70);
    });


    it('should evaluate FEEL temporal literals', async function() {

      // when
      const result = await engine.evaluateDecision('Valid Until');

      // then
      expect(String(result)).to.eql('2020-02-29');
    });
  });


  describe('DMN 1.4', function() {

    const inputs = { Scores: [ 40, 75, 100 ] };

    let engine, model;

    beforeEach(async function() {
      engine = new Engine();
      model = await engine.load(getFixture('versions/dmn14.dmn'));
    });


    it('should detect version', function() {
      expect(model.version).to.eql('1.4');
    });


    it('should evaluate boxed filter', async function() {
      expect(await engine.evaluateDecision('Passed', inputs)).to.eql([ 75, 100 ]);
    });


    it('should evaluate boxed for', async function() {
      expect(await engine.evaluateDecision('Doubled', inputs)).to.eql([ 80, 150, 200 ]);
    });


    it('should evaluate boxed every / some', async function() {
      expect(await engine.evaluateDecision('All Passed', inputs)).to.be.false;
      expect(await engine.evaluateDecision('Any Perfect', inputs)).to.be.true;
    });


    it('should evaluate boxed conditional', async function() {
      expect(await engine.evaluateDecision('Grade', inputs)).to.eql('pass');
      expect(await engine.evaluateDecision('Grade', { Scores: [ 10 ] })).to.eql('fail');
    });


    it('should evaluate DMN 1.4 builtins', async function() {
      expect(await engine.evaluateDecision('Summary', inputs)).to.eql({
        joined: '40, 75, 100',
        average: 71.7
      });
    });
  });


  describe('DMN 1.5', function() {

    let engine, model;

    beforeEach(async function() {
      engine = new Engine();
      model = await engine.load(getFixture('versions/dmn15.dmn'));
    });


    it('should detect version', function() {
      expect(model.version).to.eql('1.5');
    });


    it('should evaluate DMN 1.5 builtins', async function() {

      // given
      const inputs = { Team: [ 'ann', 'bob' ] };

      // then
      expect(await engine.evaluateDecision('Team Size', inputs)).to.eql(2);
      expect(await engine.evaluateDecision('In Range', inputs)).to.be.true;
      expect(await engine.evaluateDecision('Captain', inputs)).to.eql([ 'ANN', 'bob' ]);
    });


    it('should apply type constraint', async function() {

      // when
      const { value, messages } = await engine.evaluate('decision', 'Team Size', {
        Team: [ 'ann', 'bob', 'cid', 'dan' ]
      });

      // then
      expect(value).to.be.null;
      expect(messages.some(m => m.severity === 'error' && m.elementName === 'Team')).to.be.true;
    });
  });


  describe('DMN 1.1 / 1.2', function() {

    it('should read DMN 1.1 as DMN 1.3', async function() {

      // when
      const model = await new Engine().load(getFixture('dmn11.dmn'));

      // then
      expect(model.version).to.eql('1.1');
    });


    it('should read DMN 1.2 as DMN 1.3', async function() {

      // given
      const xml = getFixture('versions/dmn13.dmn')
        .replaceAll(DMN13, 'http://www.omg.org/spec/DMN/20180521/MODEL/')
        .replaceAll('https://www.omg.org/spec/DMN/20191111/DMNDI/', 'http://www.omg.org/spec/DMN/20180521/DMNDI/');

      const engine = new Engine();

      // when
      const model = await engine.load(xml);

      // then
      expect(model.version).to.eql('1.2');
      expect(await engine.evaluateDecision('Price', { Age: 70 })).to.eql(70);
    });
  });


  describe('validation', function() {

    it('should reject DMN 1.4 elements in DMN 1.3 model', async function() {

      // given
      const xml = getFixture('versions/dmn14.dmn').replaceAll(DMN14, DMN13);

      // then
      await expectLoadError(xml, /unparsable content <dmn:filter>|unparsable content <filter>/);
    });


    it('should reject DMN 1.5 elements in DMN 1.4 model', async function() {

      // given
      const xml = getFixture('versions/dmn15.dmn').replaceAll(DMN15, DMN14);

      // then
      await expectLoadError(xml, /unparsable content <typeConstraint>/);
    });


    it('should reject unknown DMN namespace', async function() {

      // given
      const xml = getFixture('versions/dmn15.dmn').replaceAll(DMN15, 'https://www.omg.org/spec/DMN/20990101/MODEL/');

      // then
      await expectLoadError(xml, /unsupported DMN namespace <https:\/\/www.omg.org\/spec\/DMN\/20990101\/MODEL\/>/);
    });


    it('should reject non-DMN document', async function() {
      await expectLoadError('<?xml version="1.0"?><foo />', /not a DMN document/);
    });
  });


  describe('namespace detection', function() {

    it('should read default namespace', function() {
      expect(getDefinitionsNamespace(`<definitions xmlns="${ DMN13 }" />`)).to.eql(DMN13);
    });


    it('should read prefixed namespace', function() {
      expect(getDefinitionsNamespace(
        `<!-- <definitions xmlns="foo"> --><semantic:definitions xmlns="bar" xmlns:semantic='${ DMN15 }'>`
      )).to.eql(DMN15);
    });
  });
});


async function expectLoadError(xml, pattern) {

  let error;

  try {
    await new Engine().load(xml);
  } catch (e) {
    error = e;
  }

  expect(error, 'expected load to fail').to.exist;
  expect(error.message).to.match(pattern);
}
