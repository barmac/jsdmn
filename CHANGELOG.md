# Changelog

All notable changes to [jsdmn](https://github.com/barmac/jsdmn) are documented here. We use [semantic versioning](http://semver.org/) for releases.

## Unreleased

___Note:__ Yet to be released changes appear here._

* `FEAT`: support DMN 1.3, 1.4 and 1.5 models, parsing each version strictly with its own schema
* `FEAT`: read DMN 1.1 and 1.2 models as DMN 1.3
* `FEAT`: expose the DMN version of loaded models as `Model#version`
* `FEAT`: evaluate decision tables, invocations, lists, relations, conditionals, filters and iterators
* `FEAT`: evaluate decision services and business knowledge models, directly or as functions
* `FEAT`: support imports of DMN models
* `FEAT`: use values provided for required decisions instead of evaluating them
* `FEAT`: type check and coerce values according to item definitions
* `FEAT`: evaluate FEEL with `feelin`, including `instance of` checks against item definitions
* `FEAT`: report evaluation messages via `Engine#evaluate`, with errors explaining `null` results
* `FEAT`: add `Engine#evaluateDecisionService` and `Engine#evaluateBusinessKnowledgeModel`
* `TEST`: run DMN TCK via `npm run tck`, as DMN 1.3, 1.4 and 1.5, against the TCK's latest revision
* `TEST`: record TCK test cases failing because of gaps in `feelin` as known failures
* `DEPS`: update to `feelin@8` and `dmn-moddle@12`
* `CHORE`: require Node.js >= 22

## 0.1.0

* `FEAT`: initial release :tada:
