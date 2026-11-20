import { Database } from 'bun:sqlite';
import { expect, test } from 'bun:test';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { SQLCommand, SQLValue } from './app.contract.d.ts';
import { initialize, library, openCast, readCast, recordCast, updateNotes, setOpenLine } from './repository.ts';

function database(filename = ':memory:') {
  const sql = new Database(filename);
  sql.exec('PRAGMA foreign_keys = ON');
  const execute = (text: string, params: SQLValue[] = []) => {
    const { changes, lastInsertRowid } = sql.query(text).run(...params);
    return { changes, lastInsertRowid: BigInt(lastInsertRowid) };
  };
  return {
    close: () => sql.close(),
    db: {
      async query(text: string, params: SQLValue[] = []) {
        const statement = sql.query(text);
        return { rows: statement.values(...params) as SQLValue[][], columns: statement.columnNames };
      },
      async execute(text: string, params: SQLValue[] = []) { return execute(text, params); },
      async transaction(commands: SQLCommand[]) {
        return sql.transaction(() => commands.map(c => execute(c.sql, c.params)))();
      },
    },
  };
}

const fixture = {
  seed: '2026-06-12T21:51:54+00:00', lines: [8, 8, 8, 7, 9, 8],
  changing_lines: [5], fingerprint: 'v2_ich_45.c5', primary: { number: 45 },
  resulting: { number: 16 }, nuclear: { number: 53 },
};

test('the cast, notes, branches and last-open selection survive closing the database', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'ansuz-v1-'));
  const file = join(dir, 'casts.db');
  let conn = database(file);
  try {
    const db = conn.db;
    await initialize(db);
    expect(await library(db)).toEqual({ selectedId: '', casts: [] });
    const parent = await recordCast(db, fixture, 'Original question', '');
    expect(parent).toMatch(/^[a-f0-9]{32}$/);
    const child = await recordCast(db, { ...fixture, seed: '2026-06-12T21:51:55+00:00' }, 'First follow-up', parent);
    const sibling = await recordCast(db, { ...fixture, seed: '2026-06-12T21:51:56+00:00' }, 'Second follow-up', parent);
    expect((await library(db)).selectedId).toBe(sibling);
    await updateNotes(db, parent, 'My note.');
    await setOpenLine(db, parent, 5);
    await expect(setOpenLine(db, child, 2)).rejects.toThrow('not changing');
    await openCast(db, parent);
    conn.close();
    conn = database(file);
    await initialize(conn.db);
    const saved = await readCast(conn.db, parent);
    expect(saved).toMatchObject({ id: parent, question: 'Original question', notes: 'My note.', openLine: 5, parentId: '', seed: fixture.seed, primary: 45, resulting: 16, nuclear: 53, changing: [5] });
    expect((await conn.db.query('SELECT lines, fingerprint FROM casts WHERE id = ?', [parent])).rows[0]).toEqual([JSON.stringify(fixture.lines), fixture.fingerprint]);
    const journal = await library(conn.db);
    expect(journal.selectedId).toBe(parent);
    expect(journal.casts.map(c => [c.id, c.parentId])).toEqual([[sibling, parent], [child, parent], [parent, '']]);
    expect(await recordCast(conn.db, fixture, 'Original question', '')).toBe(parent);
    expect((await library(conn.db)).casts).toHaveLength(3);
    expect((await readCast(conn.db, parent)).notes).toBe('My note.');
    await openCast(conn.db, child);
    expect((await readCast(conn.db, child)).openLine).toBe(0);
    expect((await readCast(conn.db, parent)).openLine).toBe(5);
    await setOpenLine(conn.db, parent, 0);
    expect((await readCast(conn.db, parent)).openLine).toBe(0);
    await expect(recordCast(conn.db, fixture, 'Different question', '')).rejects.toThrow('already saved');
    await expect(recordCast(conn.db, fixture, 'Missing parent', 'missing')).rejects.toThrow('could not be found');
    await expect(updateNotes(conn.db, 'missing', '')).rejects.toThrow('not found');
    expect((await library(conn.db)).selectedId).toBe(child);
  } finally { conn.close(); rmSync(dir, { recursive: true, force: true }); }
});

test('a failed selection rolls back the cast; a newer database version is preserved', async () => {
  const { db, close } = database();
  try {
    await initialize(db);
    await db.execute("CREATE TRIGGER fail_selection BEFORE INSERT ON reader_state BEGIN SELECT RAISE(ABORT, 'selection failure'); END");
    await expect(recordCast(db, fixture, '', '')).rejects.toThrow('selection failure');
    expect((await library(db)).casts).toHaveLength(0);
    await db.execute('PRAGMA user_version = 2');
    await expect(initialize(db)).rejects.toThrow('newer version');
    expect((await db.query('PRAGMA user_version')).rows).toEqual([[2]]);
  } finally { close(); }
});
