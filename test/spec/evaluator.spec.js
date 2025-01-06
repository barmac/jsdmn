import { Loader } from '../../src/loader.js';
import { evaluate } from '../../src/evaluator.js';
import { getFixture } from '../helper.js';

describe('evaluator', function() {

  it('should evaluate a decision with literal expression', async function() {

    // given
    const loader = new Loader();
    const dmn = getFixture('simple.dmn');
    await loader.load(dmn);
    const decision = loader.getDecision('Decision');

    // when
    const result = await evaluate(decision);

    // then
    expect(result).to.eql(4);
  });
});