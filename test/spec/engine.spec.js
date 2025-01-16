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
});