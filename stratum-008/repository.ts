// Ansuz v1 stores the cast once. Lenses and screen state can evolve independently.
import type { Database } from './app.contract.d.ts';

export type Db = Pick<Database, 'query' | 'execute' | 'transaction'>;
export interface CastFacts {
  seed: string;
  lines: number[];
  changing_lines: number[];
  fingerprint: string;
  primary: { number: number };
  resulting: { number: number } | null;
  nuclear: { number: number };
}

// Presentation state is keyed by a cast but remains disposable across UI strata.
const passagesTable = `CREATE TABLE IF NOT EXISTS reading_passages (
  cast_id TEXT PRIMARY KEY NOT NULL REFERENCES casts(id),
  open_line INTEGER NOT NULL CHECK (open_line BETWEEN 0 AND 6)
)`;

export async function initialize(db: Db) {
  const { rows } = await db.query('PRAGMA user_version');
  const version = Number(rows[0][0]);
  if (version === 1) {
    const table = await db.query("SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'reading_passages'");
    if (!table.rows.length) await db.execute(passagesTable);
    return;
  }
  if (version !== 0) throw new Error('This cast library requires a newer version of Ansuz.');
  await db.transaction([
    { sql: `CREATE TABLE IF NOT EXISTS casts (
      id TEXT PRIMARY KEY NOT NULL,
      seed TEXT UNIQUE NOT NULL,
      cast_at INTEGER NOT NULL,
      question TEXT NOT NULL,
      parent_id TEXT REFERENCES casts(id),
      lines TEXT NOT NULL,
      changing_lines TEXT NOT NULL,
      primary_number INTEGER NOT NULL CHECK (primary_number BETWEEN 1 AND 64),
      resulting_number INTEGER CHECK (resulting_number BETWEEN 1 AND 64),
      nuclear_number INTEGER NOT NULL CHECK (nuclear_number BETWEEN 1 AND 64),
      fingerprint TEXT NOT NULL,
      notes TEXT NOT NULL DEFAULT ''
    )` },
    { sql: 'CREATE INDEX IF NOT EXISTS cast_time ON casts(cast_at DESC, id DESC)' },
    { sql: 'CREATE INDEX IF NOT EXISTS cast_parent ON casts(parent_id)' },
    { sql: 'CREATE TABLE IF NOT EXISTS reader_state (singleton INTEGER PRIMARY KEY CHECK (singleton = 1), selected_id TEXT NOT NULL REFERENCES casts(id))' },
    { sql: passagesTable },
    { sql: 'PRAGMA user_version = 1' },
  ]);
}

export async function recordCast(db: Db, cast: CastFacts, question: string, parentId: string) {
  const castAt = Date.parse(cast.seed);
  if (!Number.isFinite(castAt)) throw new Error('The engine returned an invalid cast time.');
  if (parentId && !(await db.query('SELECT 1 FROM casts WHERE id = ?', [parentId])).rows.length) {
    throw new Error('The reading you are continuing could not be found.');
  }
  // The engine's normalized instant identifies a retried cast. Recover its ID
  // only if its original question and parent match this request.
  await db.transaction([
    {
      sql: `INSERT INTO casts(id, seed, cast_at, question, parent_id, lines, changing_lines,
                primary_number, resulting_number, nuclear_number, fingerprint)
            VALUES (lower(hex(randomblob(16))), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(seed) DO NOTHING`,
      params: [cast.seed, castAt, question, parentId || null, JSON.stringify(cast.lines),
        JSON.stringify(cast.changing_lines), cast.primary.number, cast.resulting?.number ?? null,
        cast.nuclear.number, cast.fingerprint],
    },
    {
      sql: `INSERT INTO reader_state(singleton, selected_id)
            SELECT 1, id FROM casts WHERE seed = ? AND question = ? AND parent_id IS ?
            ON CONFLICT(singleton) DO UPDATE SET selected_id = excluded.selected_id`,
      params: [cast.seed, question, parentId || null],
    },
  ]);
  const { rows } = await db.query('SELECT id FROM casts WHERE seed = ? AND question = ? AND parent_id IS ?', [cast.seed, question, parentId || null]);
  if (!rows.length) throw new Error('Another reading is already saved for that moment. Cast again for a new moment.');
  return String(rows[0][0]);
}

export async function library(db: Db) {
  const selected = await db.query('SELECT selected_id FROM reader_state WHERE singleton = 1');
  const { rows } = await db.query('SELECT id, question, parent_id, primary_number FROM casts ORDER BY cast_at DESC, id DESC');
  return {
    selectedId: String(selected.rows[0]?.[0] ?? ''),
    casts: rows.map(([id, question, parent, primary]) => ({
      id: String(id), question: String(question), parentId: String(parent ?? ''), primary: Number(primary),
    })),
  };
}

export async function readCast(db: Db, id: string) {
  const { rows } = await db.query(`SELECT seed, question, notes, parent_id, primary_number,
    resulting_number, nuclear_number, changing_lines, COALESCE(reading_passages.open_line, 0)
    FROM casts LEFT JOIN reading_passages ON reading_passages.cast_id = casts.id WHERE casts.id = ?`, [id]);
  if (!rows.length) throw new Error('Reading not found.');
  const [seed, question, notes, parent, primary, resulting, nuclear, changing, openLine] = rows[0];
  return {
    id, seed: String(seed), question: String(question), notes: String(notes), parentId: String(parent ?? ''),
    primary: Number(primary), resulting: Number(resulting ?? 0), nuclear: Number(nuclear),
    changing: JSON.parse(String(changing)) as number[], openLine: Number(openLine),
  };
}

export async function openCast(db: Db, id: string) {
  const result = await db.execute(`INSERT INTO reader_state(singleton, selected_id)
    SELECT 1, id FROM casts WHERE id = ? ON CONFLICT(singleton) DO UPDATE SET selected_id = excluded.selected_id`, [id]);
  if (!result.changes) throw new Error('Reading not found.');
  return id;
}

export async function updateNotes(db: Db, id: string, notes: string) {
  const result = await db.execute('UPDATE casts SET notes = ? WHERE id = ?', [notes, id]);
  if (!result.changes) throw new Error('Reading not found.');
  return id;
}

export async function setOpenLine(db: Db, id: string, position: number) {
  const reading = await readCast(db, id);
  if (position !== 0 && !reading.changing.includes(position)) {
    throw new Error('That line is not changing in this reading.');
  }
  await db.execute(`INSERT INTO reading_passages(cast_id, open_line) VALUES (?, ?)
    ON CONFLICT(cast_id) DO UPDATE SET open_line = excluded.open_line`, [id, position]);
  return id;
}
