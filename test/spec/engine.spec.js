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
});