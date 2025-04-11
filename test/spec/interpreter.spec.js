import { interpret } from '../../src/interpreter.js';
import { getFixture } from '../helper.js';

describe('interpreter', function() {

  it('should evaluate a decision with literal expression', async function() {

    // given
    const dmn = getFixture('simple.dmn');

    // when
    const result = await interpret(dmn, 'Decision', {});

    // then
    expect(result).to.eql(4);
  });


  it('should evaluate context', async function() {

    // given
    const dmn = getFixture('context.dmn');

    // when
    const result = await interpret(dmn, 'Decision', {});

    // then
    expect(result).to.eql({ expression: 4 });
  });
});