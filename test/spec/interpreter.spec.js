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

  describe('context', function() {

    it('should evaluate simple context', async function() {

      // given
      const dmn = getFixture('context.dmn');

      // when
      const result = await interpret(dmn, 'Decision', {});

      // then
      expect(result).to.eql({ expression: 4 });
    });


    it('should evaluate context with entries depending on each other', async function() {

      // given
      const dmn = getFixture('context.dmn');

      // when
      const result = await interpret(dmn, 'Decision with multiple entries', {});

      // then
      expect(result).to.eql({
        first: 4,
        second: 6
      });
    });


    it('should evaluate context with result', async function() {

      // given
      const dmn = getFixture('context.dmn');

      // when
      const result = await interpret(dmn, 'Context with result', {});

      // then
      expect(result).to.eql(6);
    });
  });


  describe('function definition', function() {

    it('should evaluate function definition', async function() {

      // given
      const dmn = getFixture('functionDefinition.dmn');

      // when
      const result = await interpret(dmn, 'Decision', {});

      // then
      expect(result).to.eql(8);
    });
  });

});