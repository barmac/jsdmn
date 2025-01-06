import { Loader } from '../../src/loader.js';
import { getFixture } from '../helper.js';

describe('loader', function() {

  it('should load a simple DMN', async function() {

    // given
    const loader = new Loader();
    const dmn = getFixture('simple.dmn');

    // when
    await loader.load(dmn);

    // then
    expect(loader.getDecision('Decision')).to.exist;
  });
});