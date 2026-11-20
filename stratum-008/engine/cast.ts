import figures from './figures.json';
import wilhelm from './wilhelm-de.json';
import type { CastResponse } from './engine-types.ts';

export type { CastResponse } from './engine-types.ts';

interface CastingModule {
  readonly available: boolean;
  later(request: { op: 'cast'; seed: string }): Promise<unknown>;
}

export async function cast(native: CastingModule, seed: string): Promise<CastResponse> {
  if (!native?.available) throw new Error('The on-device casting engine is unavailable.');
  // Jianyi normalizes the timestamp. Question, lens, notes and parentage are
  // reading metadata and are never sent through the casting boundary.
  return await native.later({ op: 'cast', seed }) as CastResponse;
}

type Unit = string | { text: string; commentary?: string };
type Passage = { judgment: Unit; image?: Unit; lines: Record<string, Unit> };
const passages: Record<string, Passage> = wilhelm.hexagrams;

function unit(value: Unit) {
  return typeof value === 'string'
    ? { text: value, commentary: '' }
    : { text: value.text, commentary: value.commentary ?? '' };
}

export function describe(number: number) {
  const figure = figures[number - 1];
  const passage = passages[String(number)];
  if (!figure || figure.number !== number || !passage) throw new Error('Unknown hexagram number.');
  const judgment = unit(passage.judgment);
  const image = unit(passage.image ?? '');
  return {
    number,
    title: `${number} · ${figure.pinyin} · ${figure.english}`,
    glyph: String.fromCodePoint(0x4dc0 + number - 1),
    chinese: figure.chinese,
    binary: figure.binary,
    judgment: judgment.text,
    judgmentCommentary: judgment.commentary,
    image: image.text,
    imageCommentary: image.commentary,
    lines: Array.from({ length: 6 }, (_, i) => ({
      position: i + 1,
      ...unit(passage.lines[String(i + 1)]),
    })),
    attribution: wilhelm.attribution,
  };
}
