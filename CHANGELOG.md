# Changelog

All notable changes to [jsdmn](https://github.com/barmac/jsdmn) are documented here. We use [semantic versioning](http://semver.org/) for releases.

## Unreleased

___Note:__ Yet to be released changes appear here._

* `FEAT`: pass all compliance level 2 and 3 test cases of the DMN TCK
* `FEAT`: support DMN 1.1 - 1.5 models
* `FEAT`: evaluate decision tables, invocations, lists, relations, conditionals, filters and iterators
* `FEAT`: evaluate decision services and business knowledge models, directly or as functions
* `FEAT`: support imports of DMN models
* `FEAT`: type check and coerce values according to item definitions
* `FEAT`: support Java function definitions
* `FEAT`: add FEEL interpreter implementing DMN 1.5 semantics
* `FEAT`: report evaluation messages via `Engine#evaluate`
* `FEAT`: add `Engine#evaluateDecisionService` and `Engine#evaluateBusinessKnowledgeModel`
* `TEST`: run DMN TCK via `npm run tck`
* `DEPS`: update to `feelin@8` and `dmn-moddle@12`
* `DEPS`: add `temporal-polyfill`
* `CHORE`: require Node.js >= 22

## 0.1.0

* `FEAT`: initial release :tada:
