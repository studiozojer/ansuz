import type { Answer, Sources } from './app.contract.d.ts';
import { loadReading } from './reading.ts';

const origin = import.meta.env.ANSUZ_PREVIEW_ORIGIN ?? 'http://127.0.0.1:4417';
export const grants = `net.fetch ${origin}`;

const sources: Sources = {
  reading: () => loadReading(origin),
};

export const answer: Answer = (source, args, store, storage, native) =>
  sources[source](args, store, storage, native);
