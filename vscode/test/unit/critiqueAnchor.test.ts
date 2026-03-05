import * as assert from 'assert';
import { findAnchorLine, normalizeLine, lineSimilarity, ANCHOR_THRESHOLD } from '../../src/editor/critiqueComments';

describe('findAnchorLine', () => {
  const makeDoc = (lines: string[]) => lines;

  it('exact match at original position', () => {
    const doc = makeDoc([
      'function foo() {',
      '  const x = 1;',      // line 2 — critiqued
      '  return x;',
      '}',
    ]);
    const result = findAnchorLine(doc, 2, ['  const x = 1;'], [], []);
    assert.strictEqual(result.line, 2);
    assert.ok(result.score >= ANCHOR_THRESHOLD);
    assert.strictEqual(result.outdated, false);
  });

  it('re-anchors after lines inserted above', () => {
    // Critique was on line 2. 10 lines inserted above → now line 12.
    const prefix = Array.from({ length: 10 }, (_, i) => `// added line ${i + 1}`);
    const doc = makeDoc([
      ...prefix,
      'function foo() {',
      '  const x = 1;',      // now line 12
      '  return x;',
      '}',
    ]);
    const result = findAnchorLine(doc, 2, ['  const x = 1;'], ['function foo() {'], ['  return x;']);
    assert.strictEqual(result.line, 12);
    assert.ok(!result.outdated);
  });

  it('fuzzy match with whitespace changes', () => {
    const doc = makeDoc([
      'function foo() {',
      '  const   x   =   1;',  // reformatted whitespace
      '  return x;',
    ]);
    // Stored context has original spacing
    const result = findAnchorLine(doc, 2, ['  const x = 1;'], [], []);
    assert.strictEqual(result.line, 2);
    assert.ok(!result.outdated);
  });

  it('tiebreak: picks candidate closest to original line', () => {
    // Same block appears twice; original was at line 2
    const doc = makeDoc([
      '  const x = 1;',   // line 1 — duplicate
      '  const x = 1;',   // line 2 — original position
    ]);
    const result = findAnchorLine(doc, 2, ['  const x = 1;'], [], []);
    assert.strictEqual(result.line, 2);
  });

  it('marks outdated when code is deleted', () => {
    const doc = makeDoc([
      'function bar() {',
      '  return 42;',
    ]);
    const result = findAnchorLine(doc, 5, ['  const x = complex_thing();'], [], []);
    assert.ok(result.outdated);
    assert.ok(result.score < ANCHOR_THRESHOLD);
  });

  it('empty context returns outdated', () => {
    const doc = makeDoc(['anything']);
    const result = findAnchorLine(doc, 1, [], [], []);
    assert.ok(result.outdated);
  });

  it('normalizeLine collapses whitespace', () => {
    assert.strictEqual(normalizeLine('  hello   world  '), 'hello world');
  });

  it('lineSimilarity: exact match', () => {
    assert.strictEqual(lineSimilarity('  foo()', '  foo()'), 1.0);
  });

  it('lineSimilarity: substring match', () => {
    assert.ok(lineSimilarity('  return x;', 'return x') >= 0.8);
  });

  it('lineSimilarity: no match', () => {
    assert.strictEqual(lineSimilarity('completely different', 'nothing alike here'), 0.0);
  });
});
