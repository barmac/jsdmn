import { evaluate, unaryTest } from '../../src/feel/interpreter.js';

import { isDuration, isTemporal } from '../../src/feel/temporal.js';

/**
 * Evaluate an expression, representing temporal values as strings.
 */
function feel(expression, context) {
  const { value } = evaluate(expression, context);

  return isTemporal(value) || isDuration(value) ? String(value) : value;
}

describe('feel', function() {

  describe('arithmetic', function() {

    it('numbers', function() {
      expect(feel('1 + 2 * 3')).to.eql(7);
      expect(feel('2 ** 10')).to.eql(1024);
      expect(feel('10 / 4')).to.eql(2.5);
      expect(feel('-(1 - 3)')).to.eql(2);
    });


    it('division by zero', function() {
      expect(feel('1 / 0')).to.be.null;
    });


    it('decimal like equality', function() {
      expect(feel('0.1 + 0.2 = 0.3')).to.be.true;
    });


    it('strings', function() {
      expect(feel('"foo" + "bar"')).to.eql('foobar');
    });


    it('null propagation', function() {
      expect(feel('1 + null')).to.be.null;
      expect(feel('null * 2')).to.be.null;
    });


    it('invalid operands', function() {
      expect(feel('"1" + 1')).to.be.null;
      expect(feel('@"2021-01-01" / @"2021-01-01"')).to.be.null;
    });
  });


  describe('temporal', function() {

    it('date and durations', function() {
      expect(feel('@"2021-01-31" + @"P1M"')).to.eql('2021-02-28');
      expect(feel('@"2021-01-02" - @"PT1H"')).to.eql('2021-01-01');
      expect(feel('@"2021-01-02" - @"2021-01-01"')).to.eql('P1D');
    });


    it('date and time arithmetic', function() {
      expect(feel('@"2021-01-01T24:00:00" + @"PT1S"')).to.eql('2021-01-02T00:00:01');
      expect(feel('@"2021-01-02T10:10:10+01:00" - @"2021-01-01"')).to.eql('P1DT9H10M10S');
      expect(feel('@"2021-01-01T00:00:00" - @"2021-01-02"')).to.be.null;
    });


    it('duration arithmetic', function() {
      expect(feel('2 * @"P1DT1H"')).to.eql('P2DT2H');
      expect(feel('@"P1Y" / 2')).to.eql('P6M');
      expect(feel('@"P2D" / @"P1D"')).to.eql(2);
      expect(feel('-@"P1Y"')).to.eql('-P1Y');
    });


    it('negative and extreme years', function() {
      expect(feel('string(@"-2021-01-01T10:10:10+11:00" - @"P1D")')).to.eql('-2022-12-31T10:10:10+11:00');
      expect(feel('string(date(999999999, 12, 31))')).to.eql('999999999-12-31');
    });


    it('equality', function() {
      expect(feel('@"2002-04-02T12:00:00-01:00" = @"2002-04-02T17:00:00+04:00"')).to.be.true;
      expect(feel('duration("P1Y") = duration("P12M")')).to.be.true;
      expect(feel('duration("P1Y") = duration("P365D")')).to.be.null;
    });


    it('properties', function() {
      expect(feel('@"2021-03-04".month')).to.eql(3);
      expect(feel('@"10:20:30.5".second')).to.eql(30.5);
      expect(feel('@"P1DT2H".hours')).to.eql(2);
    });
  });


  describe('equality', function() {

    it('should be null for different types', function() {
      expect(feel('1 = "1"')).to.be.null;
      expect(feel('[100] = 100')).to.be.null;
    });


    it('should compare deeply', function() {
      expect(feel('{a: [1, {b: 2}]} = {a: [1, {b: 2}]}')).to.be.true;
      expect(feel('[1, 2] = [2, 1]')).to.be.false;
    });


    it('should distinguish ranges', function() {
      expect(feel('[1..10] = [1..10]')).to.be.true;
      expect(feel('(< 10) = (null..10)')).to.be.false;
      expect(feel('(=10) = (=10)')).to.be.true;
    });
  });


  describe('lists and contexts', function() {

    it('filter', function() {
      expect(feel('[1, 2, 3, 4][item > 2]')).to.eql([ 3, 4 ]);
      expect(feel('[1, 2, 3][-1]')).to.eql(3);
      expect(feel('[{item: 1}, {item: 2}][item >= 2]')).to.eql([ { item: 2 } ]);
    });


    it('path', function() {
      expect(feel('[{a: 1}, {a: 2}].a')).to.eql([ 1, 2 ]);
      expect(feel('{a b: {c: 1}}.a b.c')).to.eql(1);
    });


    it('context entries referring to each other', function() {
      expect(feel('{a: 1, b: a + 1}.b')).to.eql(2);
    });


    it('duplicate context keys', function() {
      expect(feel('{foo: "bar", foo: "baz"}')).to.be.null;
    });
  });


  describe('iteration', function() {

    it('for', function() {
      expect(feel('for x in [1, 2], y in [10, 20] return x * y')).to.eql([ 10, 20, 20, 40 ]);
      expect(feel('for i in 3..1 return i')).to.eql([ 3, 2, 1 ]);
      expect(feel('for i in 0..4 return if i = 0 then 1 else i * partial[-1]')).to.eql([ 1, 1, 2, 6, 24 ]);
    });


    it('for over dates', function() {
      expect(feel('for d in @"2021-01-01"..@"2021-01-02" return string(d)')).to.eql([ '2021-01-01', '2021-01-02' ]);
    });


    it('quantifiers', function() {
      expect(feel('some x in [1, 2, 3] satisfies x > 2')).to.be.true;
      expect(feel('every x in [1, 2, 3] satisfies x > 2')).to.be.false;
    });
  });


  describe('functions', function() {

    it('user defined', function() {
      expect(feel('{f: function(a, b) a - b, r: f(b: 1, a: 3)}.r')).to.eql(2);
    });


    it('recursion', function() {
      expect(feel('{fac: function(n) if n <= 1 then 1 else n * fac(n - 1), r: fac(5)}.r')).to.eql(120);
    });


    it('typed parameters', function() {
      expect(feel('{f: function(a: number) a, r: f("foo")}.r')).to.be.null;
    });


    it('external Java functions', function() {
      expect(feel('{max: function(a, b) external {java: {class: "java.lang.Math", method signature: "max(double, double)"}}, r: max(1, 2)}.r')).to.eql(2);
    });


    it('builtins', function() {
      expect(feel('string length("foo")')).to.eql(3);
      expect(feel('decimal(2.5, 0)')).to.eql(2);
      expect(feel('round half up(-1.126, 2)')).to.eql(-1.13);
      expect(feel('matches("hello world", "hello\\ sworld", "x")')).to.be.true;
      expect(feel('during(5, [1..10])')).to.be.true;
      expect(feel('2 in range("[1..3]")')).to.be.true;
      expect(feel('list replace([1, 2, 3], 2, 4)')).to.eql([ 1, 4, 3 ]);
    });
  });


  describe('instance of', function() {

    it('builtin types', function() {
      expect(feel('1 instance of number')).to.be.true;
      expect(feel('[1, 2] instance of list<number>')).to.be.true;
      expect(feel('{a: "x"} instance of context<a: string>')).to.be.true;
      expect(feel('[1..2] instance of range<number>')).to.be.true;
      expect(feel('null instance of Any')).to.be.false;
    });


    it('function types', function() {
      expect(feel('(function(a: number) a) instance of function<number>->Any')).to.be.true;
    });
  });


  describe('unary tests', function() {

    function satisfies(text, input, context) {
      return unaryTest(text, input, context).value;
    }


    it('simple', function() {
      expect(satisfies('< 10', 5)).to.be.true;
      expect(satisfies('[1..5)', 5)).to.be.false;
      expect(satisfies('"a", "b"', 'b')).to.be.true;
      expect(satisfies('-', 'anything')).to.be.true;
    });


    it('negation', function() {
      expect(satisfies('not("a", "b")', 'c')).to.be.true;
      expect(satisfies('not(< 5)', 3)).to.be.false;
    });


    it('expressions with input', function() {
      expect(satisfies('? > x', 5, { x: 3 })).to.be.true;
      expect(satisfies('even(?)', 4)).to.be.true;
    });


    it('lists and ranges', function() {
      expect(satisfies('[1, 2, 3]', 2)).to.be.true;
      expect(satisfies('x', 5, { x: [ 1, 5 ] })).to.be.true;
    });


    it('null semantics', function() {
      expect(satisfies('null', null)).to.be.true;
      expect(satisfies('(null..10]', 5)).to.be.null;
    });
  });


  describe('errors', function() {

    it('should throw on syntax errors', function() {
      expect(() => evaluate('1 +')).to.throw(/syntax error/);
    });


    it('should report invalid operations', function() {
      expect(evaluate('null.b').warnings[0]).to.include({ type: 'NO_PROPERTY_FOUND' });
      expect(evaluate('1 + null').warnings[0]).to.include({ type: 'INVALID_TYPE' });
      expect(evaluate('null between 1 and 10').warnings[0]).to.include({ type: 'NOT_COMPARABLE' });
      expect(evaluate('modulo(10, null)').warnings[0]).to.include({ type: 'INVALID_ARGUMENTS' });
      expect(evaluate('not(null)').warnings).to.be.empty;
    });


    it('should report warnings', function() {

      // when
      const { value, warnings } = evaluate('unknown + 1');

      // then
      expect(value).to.be.null;
      expect(warnings[0]).to.include({ type: 'NO_VARIABLE_FOUND' });
    });
  });
});
