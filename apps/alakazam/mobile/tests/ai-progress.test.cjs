const assert = require('node:assert/strict');
const { test } = require('node:test');
const { loadTs } = require('./load-ts.cjs');

const load = () =>
  loadTs('state/ai-progress.ts', { rxjs: require('rxjs'), 'rxjs/operators': require('rxjs/operators') }, { __DEV__: false });
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));

test('every task is worth the same share; a finished one counts all of it, a silent one counts the guess given for it', () => {
  const { overallProgress } = load();
  assert.deepEqual(overallProgress({}), { running: false, share: 0, unsure: false });
  assert.equal(overallProgress({ faces: 0.5 }).share, 0.5, 'one task is its own progress');
  assert.equal(overallProgress({ skin: 1, faces: 0.5 }).share, 0.75, 'the average of the tasks');
  assert.equal(overallProgress({ skin: 1, people: 0.1 }).share, 0.55, 'a finished task stays in the total, so the bar does not drop');
  const mixed = overallProgress({ skin: null, faces: 0.5 });
  assert.equal(mixed.share, 0.25, 'a task that does not report counts nothing without a guess');
  assert.equal(mixed.unsure, true, 'and the bar says part of it is a guess');
  assert.equal(overallProgress({ skin: null, faces: 0.5 }, 0.5).share, 0.5, 'and what it is guessed to be with one');
  assert.equal(overallProgress({ skin: null, people: null }).share, 0, 'a job of silent tasks starts at nothing');
  assert.equal(overallProgress({ skin: 1, faces: 1 }).unsure, false);
});

test('a job that is finished clears after a moment, and one that gets a new task meanwhile does not', async () => {
  const { aiProgress } = load();
  aiProgress.next({ skin: null });
  aiProgress.next({ skin: 1 });
  await wait(700);
  assert.deepEqual(aiProgress.value, {}, 'cleared once everything has finished');

  aiProgress.next({ skin: 1 });
  await wait(250);
  aiProgress.next({ skin: 1, people: null });
  await wait(700);
  assert.deepEqual(aiProgress.value, { skin: 1, people: null }, 'a task that started in time keeps the job');
  aiProgress.next({ skin: 1, people: 1 });
  await wait(700);
  assert.deepEqual(aiProgress.value, {}, 'and the job clears when that one ends');
});
