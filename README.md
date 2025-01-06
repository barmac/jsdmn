# jsdmn

[![CI](https://github.com/barmac/jsdmn/workflows/CI/badge.svg)](https://github.com/barmac/jsdmn/actions?query=workflow%3ACI)

Evaluate DMN 1.3 decisions both in NodeJS and in browser.

## Goal

This project is built for learning, so don't expect production-readiness soon (or anytime, really). Nevertheless, the end goal is to pass [the dmntck test suite](https://github.com/dmn-tck/tck).

## Usage

```javascript
import { Engine } from 'jsdmn';

const engine = new Engine();

await engine.load(dmnXML);
const result = await engine.evaluateDecision('Decision', { input: 4 });

console.log(result) // prints result
```

## Credits

This library is built upon two amazing projects:

* [feelin](https://github.com/nikku/feelin) - JavaScript FEEL interpreter
* [dmn-moddle](https://github.com/bpmn-io/dmn-moddle) - DMN parser

## License

Use under the terms of the [MIT license](http://opensource.org/licenses/MIT).
