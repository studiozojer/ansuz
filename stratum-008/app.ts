import type { Answer, Result, Sources, Storage } from './app.contract.d.ts';
import { cast, describe } from './engine/cast.ts';
import * as repository from './repository.ts';

export const appId = 'studio.zojer.ansuz.readings';
export const grants = 'sqlite.open app:/data/ansuz-casts.db';

// SQLite on the web persists to the app's device storage on close. Serialize
// each open/work/close to keep resource refreshes behind the write they follow.
let tail: Promise<unknown> = Promise.resolve();
function withDb<T>(storage: Storage, work: (db: repository.Db) => Promise<T>): Promise<T> {
  const task = tail.then(async () => {
    const db = await storage.sqlite.open('app:/data/ansuz-casts.db');
    try {
      await repository.initialize(db);
      return await work(db);
    } finally {
      await db.close();
    }
  });
  tail = task.catch(() => undefined);
  return task;
}

async function mutate(action: () => Promise<string>): Promise<Result<'recordCast'>> {
  try { return { ok: true, id: await action(), error: '' }; }
  catch (error) {
    if ((error as {code?: string})?.code === 'bake') throw error;
    return { ok: false, id: '', error: error instanceof Error ? error.message : 'The reading could not be saved.' };
  }
}

const sources: Sources = {
  loadLibrary: (_args, _store, storage) => withDb(storage, async db => {
    const { selectedId, casts } = await repository.library(db);
    return { selectedId, casts: casts.map(({ primary, ...summary }) => ({ ...summary, title: describe(primary).title })) };
  }),
  loadReading: async ([id], _store, storage) => {
    if (!id) return {
      id: '', question: '', seed: '', notes: '', parentId: '', primary: 0, resulting: 0, nuclear: 0,
      title: '', glyph: '', judgment: '', image: '', lines: [], resultingTitle: '', resultingGlyph: '',
      nuclearTitle: '', attribution: '', chinese: '', castAt: 0, figureLines: [], openLine: 0,
    };
    const { changing, ...record } = await withDb(storage, db => repository.readCast(db, id));
    const primary = describe(record.primary);
    const resulting = record.resulting ? describe(record.resulting) : null;
    return {
      ...record, title: primary.title, glyph: primary.glyph, judgment: primary.judgment,
      image: primary.image, chinese: primary.chinese, castAt: Date.parse(record.seed),
      figureLines: [...primary.binary].reverse().map((bit, i) => ({
        position: 6 - i, yang: bit === '1', changing: changing.includes(6 - i),
      })),
      lines: primary.lines.filter(line => changing.includes(line.position)),
      attribution: primary.attribution, resultingTitle: resulting?.title ?? '',
      resultingGlyph: resulting?.glyph ?? '', nuclearTitle: describe(record.nuclear).title,
    };
  },
  recordCast: ([at, question, parentId], _store, storage, native) => mutate(async () => {
    if (!native) throw new Error('The on-device casting engine is unavailable.');
    const result = await cast(native, new Date(at).toISOString());
    return withDb(storage, db => repository.recordCast(db, result, question, parentId));
  }),
  openCast: ([id], _store, storage) => mutate(() => withDb(storage, db => repository.openCast(db, id))),
  setOpenLine: ([id, position], _store, storage) => mutate(() => withDb(storage, db => repository.setOpenLine(db, id, position))),
  updateNotes: ([id, notes], _store, storage) => mutate(() => withDb(storage, db => repository.updateNotes(db, id, notes))),
};

export const answer: Answer = (source, args, store, storage, native) =>
  sources[source](args, store, storage, native);
