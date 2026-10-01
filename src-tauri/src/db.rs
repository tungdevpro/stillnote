use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};

use crate::error::{AppError, AppResult};
use crate::models::{Note, NoteFilter, NoteSummary, Notebook, RemoteNote, RemoteNotebook, TagCount};

const SCHEMA_V1: &str = r#"
CREATE TABLE notebooks (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    deleted     INTEGER NOT NULL DEFAULT 0,
    dirty       INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE notes (
    id          TEXT PRIMARY KEY,
    notebook_id TEXT,
    title       TEXT NOT NULL DEFAULT '',
    body        TEXT NOT NULL DEFAULT '',
    tags        TEXT NOT NULL DEFAULT '[]',
    pinned      INTEGER NOT NULL DEFAULT 0,
    trashed_at  INTEGER,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    deleted     INTEGER NOT NULL DEFAULT 0,
    dirty       INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX notes_updated_at ON notes(updated_at DESC);
CREATE INDEX notes_notebook ON notes(notebook_id);

CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- remove_diacritics 2: "ghi chu" matches "ghi chú".
CREATE VIRTUAL TABLE notes_fts USING fts5(
    title, body,
    content='notes', content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);
CREATE TRIGGER notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, title, body) VALUES (new.rowid, new.title, new.body);
END;
CREATE TRIGGER notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body) VALUES ('delete', old.rowid, old.title, old.body);
END;
CREATE TRIGGER notes_au AFTER UPDATE OF title, body ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body) VALUES ('delete', old.rowid, old.title, old.body);
    INSERT INTO notes_fts(rowid, title, body) VALUES (new.rowid, new.title, new.body);
END;
"#;

const MIGRATIONS: &[&str] = &[SCHEMA_V1];

const NOTE_COLUMNS: &str =
    "id, notebook_id, title, body, tags, pinned, trashed_at, created_at, updated_at";

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&mut self) -> AppResult<()> {
        let version: i64 = self.conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i as i64 + 1)?;
            tx.commit()?;
        }
        Ok(())
    }

    // ---------- meta ----------

    pub fn get_meta(&self, key: &str) -> AppResult<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    pub fn delete_meta(&self, key: &str) -> AppResult<()> {
        self.conn.execute("DELETE FROM meta WHERE key = ?1", [key])?;
        Ok(())
    }

    // ---------- notebooks ----------

    pub fn list_notebooks(&self) -> AppResult<Vec<Notebook>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.id, b.name, b.created_at, b.updated_at,
                    (SELECT COUNT(*) FROM notes n
                     WHERE n.notebook_id = b.id AND n.deleted = 0 AND n.trashed_at IS NULL)
             FROM notebooks b
             WHERE b.deleted = 0
             ORDER BY b.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Notebook {
                id: r.get(0)?,
                name: r.get(1)?,
                created_at: r.get(2)?,
                updated_at: r.get(3)?,
                note_count: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn create_notebook(&self, name: &str) -> AppResult<Notebook> {
        let name = clean_name(name)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO notebooks (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![id, name, now],
        )?;
        Ok(Notebook { id, name, note_count: 0, created_at: now, updated_at: now })
    }

    pub fn rename_notebook(&self, id: &str, name: &str) -> AppResult<()> {
        let name = clean_name(name)?;
        let changed = self.conn.execute(
            "UPDATE notebooks SET name = ?2, updated_at = ?3, dirty = 1
             WHERE id = ?1 AND deleted = 0",
            params![id, name, now_ms()],
        )?;
        if changed == 0 {
            return Err(AppError::NotFound("notebook"));
        }
        Ok(())
    }

    /// Deletes a notebook and moves its notes to the trash.
    pub fn delete_notebook(&mut self, id: &str) -> AppResult<()> {
        let now = now_ms();
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE notebooks SET deleted = 1, updated_at = ?2, dirty = 1
             WHERE id = ?1 AND deleted = 0",
            params![id, now],
        )?;
        if changed == 0 {
            return Err(AppError::NotFound("notebook"));
        }
        tx.execute(
            "UPDATE notes SET notebook_id = NULL,
                    trashed_at = COALESCE(trashed_at, ?2), updated_at = ?2, dirty = 1
             WHERE notebook_id = ?1 AND deleted = 0",
            params![id, now],
        )?;
        tx.commit()?;
        Ok(())
    }

    // ---------- tags ----------

    pub fn list_tags(&self) -> AppResult<Vec<TagCount>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.value, COUNT(*)
             FROM notes n, json_each(n.tags) t
             WHERE n.deleted = 0 AND n.trashed_at IS NULL
             GROUP BY t.value
             ORDER BY t.value COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| Ok(TagCount { name: r.get(0)?, count: r.get(1)? }))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    // ---------- notes ----------

    pub fn list_notes(&self, filter: &NoteFilter) -> AppResult<Vec<NoteSummary>> {
        let fts = filter.query.as_deref().and_then(fts_query);
        let mut args: Vec<String> = Vec::new();

        let mut sql = String::from(
            "SELECT n.id, n.notebook_id, n.title, n.tags, n.pinned, n.trashed_at, n.updated_at, ",
        );
        if fts.is_some() {
            sql.push_str("snippet(notes_fts, 1, '', '', '…', 24) FROM notes n JOIN notes_fts f ON f.rowid = n.rowid WHERE notes_fts MATCH ? AND ");
            args.push(fts.unwrap());
        } else {
            sql.push_str("substr(n.body, 1, 300) FROM notes n WHERE ");
        }
        sql.push_str("n.deleted = 0 AND ");
        sql.push_str(if filter.trash { "n.trashed_at IS NOT NULL" } else { "n.trashed_at IS NULL" });
        if let Some(nb) = &filter.notebook_id {
            sql.push_str(" AND n.notebook_id = ?");
            args.push(nb.clone());
        }
        if let Some(tag) = &filter.tag {
            sql.push_str(" AND EXISTS (SELECT 1 FROM json_each(n.tags) WHERE value = ?)");
            args.push(tag.clone());
        }
        sql.push_str(if filter.trash {
            " ORDER BY n.trashed_at DESC"
        } else {
            " ORDER BY n.pinned DESC, n.updated_at DESC"
        });

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(args.iter()), |r| {
            let raw: String = r.get(7)?;
            Ok(NoteSummary {
                id: r.get(0)?,
                notebook_id: r.get(1)?,
                title: r.get(2)?,
                tags: parse_tags(&r.get::<_, String>(3)?),
                pinned: r.get(4)?,
                trashed_at: r.get(5)?,
                updated_at: r.get(6)?,
                snippet: collapse_whitespace(&raw, 160),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get_note(&self, id: &str) -> AppResult<Note> {
        self.conn
            .query_row(
                &format!("SELECT {NOTE_COLUMNS} FROM notes WHERE id = ?1 AND deleted = 0"),
                [id],
                note_from_row,
            )
            .optional()?
            .ok_or(AppError::NotFound("note"))
    }

    pub fn create_note(&self, notebook_id: Option<&str>) -> AppResult<Note> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO notes (id, notebook_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![id, notebook_id, now],
        )?;
        self.get_note(&id)
    }

    pub fn update_note_content(&self, id: &str, title: &str, body: &str) -> AppResult<Note> {
        self.touch_note(
            id,
            "title = ?2, body = ?3",
            params![id, title, body, now_ms()],
            "AND (title IS NOT ?2 OR body IS NOT ?3)",
        )
    }

    pub fn set_note_notebook(&self, id: &str, notebook_id: Option<&str>) -> AppResult<Note> {
        self.touch_note(id, "notebook_id = ?2", params![id, notebook_id, now_ms()], "")
    }

    pub fn set_note_tags(&self, id: &str, tags: &[String]) -> AppResult<Note> {
        let json = serde_json::to_string(&normalize_tags(tags))?;
        self.touch_note(id, "tags = ?2", params![id, json, now_ms()], "")
    }

    pub fn set_note_pinned(&self, id: &str, pinned: bool) -> AppResult<Note> {
        self.touch_note(id, "pinned = ?2", params![id, pinned, now_ms()], "")
    }

    pub fn trash_note(&self, id: &str) -> AppResult<Note> {
        let now = now_ms();
        self.touch_note(id, "trashed_at = ?2", params![id, now, now], "")
    }

    /// Restores a note; if its notebook is gone, it lands in "all notes".
    pub fn restore_note(&self, id: &str) -> AppResult<Note> {
        self.touch_note(
            id,
            "trashed_at = NULL,
             notebook_id = (SELECT b.id FROM notebooks b WHERE b.id = notes.notebook_id AND b.deleted = 0)",
            params![id, now_ms()],
            "",
        )
    }

    /// Permanently deletes a note. A tombstone is kept so the deletion syncs.
    pub fn delete_note_forever(&self, id: &str) -> AppResult<()> {
        let changed = self.conn.execute(
            "UPDATE notes SET deleted = 1, title = '', body = '', tags = '[]',
                    updated_at = ?2, dirty = 1
             WHERE id = ?1 AND deleted = 0",
            params![id, now_ms()],
        )?;
        if changed == 0 {
            return Err(AppError::NotFound("note"));
        }
        Ok(())
    }

    pub fn empty_trash(&self) -> AppResult<usize> {
        Ok(self.conn.execute(
            "UPDATE notes SET deleted = 1, title = '', body = '', tags = '[]',
                    updated_at = ?1, dirty = 1
             WHERE trashed_at IS NOT NULL AND deleted = 0",
            [now_ms()],
        )?)
    }

    /// Runs `UPDATE notes SET <set>, updated_at = ?N, dirty = 1 WHERE id = ?1 ...`
    /// where the last bound parameter is the timestamp.
    fn touch_note(
        &self,
        id: &str,
        set: &str,
        params: &[&dyn rusqlite::ToSql],
        extra_where: &str,
    ) -> AppResult<Note> {
        let ts = params.len();
        let sql = format!(
            "UPDATE notes SET {set}, updated_at = ?{ts}, dirty = 1
             WHERE id = ?1 AND deleted = 0 {extra_where}"
        );
        self.conn.execute(&sql, params)?;
        self.get_note(id)
    }

    // ---------- sync support ----------

    pub fn dirty_notes(&self) -> AppResult<Vec<RemoteNote>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {NOTE_COLUMNS}, deleted FROM notes WHERE dirty = 1 ORDER BY updated_at"
        ))?;
        let rows = stmt.query_map([], |r| {
            Ok(RemoteNote {
                id: r.get(0)?,
                notebook_id: r.get(1)?,
                title: r.get(2)?,
                body: r.get(3)?,
                tags: parse_tags(&r.get::<_, String>(4)?),
                pinned: r.get(5)?,
                trashed_at: r.get(6)?,
                created_at: r.get(7)?,
                updated_at: r.get(8)?,
                deleted: r.get(9)?,
                server_seq: 0,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn dirty_notebooks(&self) -> AppResult<Vec<RemoteNotebook>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, created_at, updated_at, deleted FROM notebooks
             WHERE dirty = 1 ORDER BY updated_at",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(RemoteNotebook {
                id: r.get(0)?,
                name: r.get(1)?,
                created_at: r.get(2)?,
                updated_at: r.get(3)?,
                deleted: r.get(4)?,
                server_seq: 0,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Clears the dirty flag, unless the row was edited again while pushing.
    pub fn mark_clean(&mut self, table: SyncTable, rows: &[(String, i64)]) -> AppResult<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(&format!(
                "UPDATE {} SET dirty = 0 WHERE id = ?1 AND updated_at = ?2",
                table.name()
            ))?;
            for (id, updated_at) in rows {
                stmt.execute(params![id, updated_at])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Applies rows pulled from the server. Last write (by `updated_at`) wins.
    pub fn apply_remote_notes(&mut self, rows: &[RemoteNote]) -> AppResult<usize> {
        let tx = self.conn.transaction()?;
        let mut applied = 0;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO notes (id, notebook_id, title, body, tags, pinned, trashed_at,
                                    created_at, updated_at, deleted, dirty)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)
                 ON CONFLICT(id) DO UPDATE SET
                    notebook_id = excluded.notebook_id, title = excluded.title,
                    body = excluded.body, tags = excluded.tags, pinned = excluded.pinned,
                    trashed_at = excluded.trashed_at, created_at = excluded.created_at,
                    updated_at = excluded.updated_at, deleted = excluded.deleted, dirty = 0
                 WHERE excluded.updated_at > notes.updated_at",
            )?;
            for n in rows {
                applied += stmt.execute(params![
                    n.id,
                    n.notebook_id,
                    n.title,
                    n.body,
                    serde_json::to_string(&n.tags)?,
                    n.pinned,
                    n.trashed_at,
                    n.created_at,
                    n.updated_at,
                    n.deleted,
                ])?;
            }
        }
        tx.commit()?;
        Ok(applied)
    }

    pub fn apply_remote_notebooks(&mut self, rows: &[RemoteNotebook]) -> AppResult<usize> {
        let tx = self.conn.transaction()?;
        let mut applied = 0;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO notebooks (id, name, created_at, updated_at, deleted, dirty)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name, created_at = excluded.created_at,
                    updated_at = excluded.updated_at, deleted = excluded.deleted, dirty = 0
                 WHERE excluded.updated_at > notebooks.updated_at",
            )?;
            for b in rows {
                applied += stmt.execute(params![b.id, b.name, b.created_at, b.updated_at, b.deleted])?;
            }
        }
        tx.commit()?;
        Ok(applied)
    }

    /// Marks everything for upload, e.g. after signing in to a new account.
    pub fn mark_all_dirty(&self) -> AppResult<()> {
        self.conn.execute_batch("UPDATE notes SET dirty = 1; UPDATE notebooks SET dirty = 1;")?;
        Ok(())
    }

    /// Removes all notes and notebooks from this device (settings are kept).
    pub fn wipe_local_data(&self) -> AppResult<()> {
        self.conn.execute_batch("DELETE FROM notes; DELETE FROM notebooks;")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SyncTable {
    Notes,
    Notebooks,
}

impl SyncTable {
    pub fn name(self) -> &'static str {
        match self {
            SyncTable::Notes => "notes",
            SyncTable::Notebooks => "notebooks",
        }
    }
}

fn note_from_row(r: &Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: r.get(0)?,
        notebook_id: r.get(1)?,
        title: r.get(2)?,
        body: r.get(3)?,
        tags: parse_tags(&r.get::<_, String>(4)?),
        pinned: r.get(5)?,
        trashed_at: r.get(6)?,
        created_at: r.get(7)?,
        updated_at: r.get(8)?,
    })
}

fn parse_tags(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        let t = t.trim().trim_start_matches('#').trim();
        if !t.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(t)) {
            out.push(t.to_string());
        }
    }
    out
}

fn clean_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("blankNotebookName"));
    }
    Ok(name.to_string())
}

/// Plain-text preview: Markdown markers dropped, whitespace collapsed, truncated.
fn collapse_whitespace(s: &str, max_chars: usize) -> String {
    let joined = s
        .lines()
        .map(strip_line_markers)
        .flat_map(str::split_whitespace)
        .map(|w| w.trim_matches(|c| matches!(c, '*' | '_' | '`' | '~')))
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    match joined.char_indices().nth(max_chars) {
        Some((i, _)) => format!("{}…", &joined[..i]),
        None => joined,
    }
}

fn strip_line_markers(line: &str) -> &str {
    let mut l = line.trim_start();
    l = l.trim_start_matches('#').trim_start_matches('>').trim_start();
    for bullet in ["- ", "* ", "+ "] {
        if let Some(rest) = l.strip_prefix(bullet) {
            l = rest;
            break;
        }
    }
    for task in ["[ ] ", "[x] ", "[X] "] {
        if let Some(rest) = l.strip_prefix(task) {
            l = rest;
            break;
        }
    }
    l
}

/// Turns user input into an FTS5 query: every word must match as a prefix.
fn fts_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .map(|w| w.replace('"', ""))
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{w}\"*"))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Db {
        Db::open_in_memory().unwrap()
    }

    fn all(db: &Db) -> Vec<NoteSummary> {
        db.list_notes(&NoteFilter::default()).unwrap()
    }

    #[test]
    fn create_and_edit_note() {
        let db = db();
        let note = db.create_note(None).unwrap();
        assert_eq!(note.title, "");
        let edited = db.update_note_content(&note.id, "Đi chợ", "Mua rau").unwrap();
        assert_eq!(edited.title, "Đi chợ");
        assert_eq!(edited.body, "Mua rau");
        assert_eq!(all(&db).len(), 1);
    }

    #[test]
    fn search_ignores_vietnamese_diacritics_and_matches_prefixes() {
        let db = db();
        let a = db.create_note(None).unwrap();
        db.update_note_content(&a.id, "Kế hoạch", "Họp nhóm thứ hai").unwrap();
        let b = db.create_note(None).unwrap();
        db.update_note_content(&b.id, "Công thức", "Phở bò").unwrap();

        let search = |q: &str| {
            db.list_notes(&NoteFilter { query: Some(q.into()), ..Default::default() })
                .unwrap()
                .into_iter()
                .map(|n| n.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(search("ke hoach"), vec![a.id.clone()]);
        assert_eq!(search("hop"), vec![a.id.clone()]);
        assert_eq!(search("pho"), vec![b.id.clone()]);
        assert!(search("không có").is_empty());
        assert_eq!(search("   ").len(), 2, "blank query lists everything");
        assert_eq!(search("\"quote"), Vec::<String>::new(), "quotes are stripped safely");
    }

    #[test]
    fn search_index_follows_edits() {
        let db = db();
        let n = db.create_note(None).unwrap();
        db.update_note_content(&n.id, "", "cũ").unwrap();
        db.update_note_content(&n.id, "", "mới").unwrap();
        let q = |s: &str| NoteFilter { query: Some(s.into()), ..Default::default() };
        assert!(db.list_notes(&q("cu")).unwrap().is_empty());
        assert_eq!(db.list_notes(&q("moi")).unwrap().len(), 1);
    }

    #[test]
    fn tags_are_normalized_and_filterable() {
        let db = db();
        let n = db.create_note(None).unwrap();
        let note = db
            .set_note_tags(&n.id, &["#work".into(), " Work ".into(), "ý tưởng".into(), "".into()])
            .unwrap();
        assert_eq!(note.tags, vec!["work", "ý tưởng"]);
        db.create_note(None).unwrap();

        let tags = db.list_tags().unwrap();
        assert_eq!(tags.len(), 2);
        let by_tag = db
            .list_notes(&NoteFilter { tag: Some("work".into()), ..Default::default() })
            .unwrap();
        assert_eq!(by_tag.len(), 1);
    }

    #[test]
    fn pinned_notes_come_first() {
        let db = db();
        let a = db.create_note(None).unwrap();
        let _b = db.create_note(None).unwrap();
        db.set_note_pinned(&a.id, true).unwrap();
        assert_eq!(all(&db)[0].id, a.id);
    }

    #[test]
    fn trash_restore_and_delete_forever() {
        let db = db();
        let n = db.create_note(None).unwrap();
        db.trash_note(&n.id).unwrap();
        assert!(all(&db).is_empty());
        let trash = NoteFilter { trash: true, ..Default::default() };
        assert_eq!(db.list_notes(&trash).unwrap().len(), 1);

        db.restore_note(&n.id).unwrap();
        assert_eq!(all(&db).len(), 1);

        db.trash_note(&n.id).unwrap();
        assert_eq!(db.empty_trash().unwrap(), 1);
        assert!(db.list_notes(&trash).unwrap().is_empty());
        assert!(matches!(db.get_note(&n.id), Err(AppError::NotFound(_))));
        // The tombstone is still waiting to be synced.
        assert!(db.dirty_notes().unwrap().iter().any(|r| r.id == n.id && r.deleted));
    }

    #[test]
    fn deleting_notebook_trashes_its_notes() {
        let mut db = db();
        let nb = db.create_notebook("Công việc").unwrap();
        let n = db.create_note(Some(&nb.id)).unwrap();
        assert_eq!(db.list_notebooks().unwrap()[0].note_count, 1);

        db.delete_notebook(&nb.id).unwrap();
        assert!(db.list_notebooks().unwrap().is_empty());
        let restored = db.restore_note(&n.id).unwrap();
        assert_eq!(restored.notebook_id, None);
    }

    #[test]
    fn notebook_name_must_not_be_blank() {
        assert!(matches!(db().create_notebook("  "), Err(AppError::Invalid(_))));
    }

    #[test]
    fn mark_clean_skips_rows_edited_during_push() {
        let mut db = db();
        let n = db.create_note(None).unwrap();
        let pushed = db.dirty_notes().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        db.update_note_content(&n.id, "edited", "").unwrap();
        let ids: Vec<_> = pushed.iter().map(|r| (r.id.clone(), r.updated_at)).collect();
        db.mark_clean(SyncTable::Notes, &ids).unwrap();
        assert_eq!(db.dirty_notes().unwrap().len(), 1);

        let fresh: Vec<_> = db.dirty_notes().unwrap().iter().map(|r| (r.id.clone(), r.updated_at)).collect();
        db.mark_clean(SyncTable::Notes, &fresh).unwrap();
        assert!(db.dirty_notes().unwrap().is_empty());
    }

    #[test]
    fn remote_changes_use_last_write_wins() {
        let mut db = db();
        let n = db.create_note(None).unwrap();
        let local = db.update_note_content(&n.id, "local", "").unwrap();

        let mut remote = db.dirty_notes().unwrap().remove(0);
        remote.title = "older remote".into();
        remote.updated_at = local.updated_at - 1000;
        assert_eq!(db.apply_remote_notes(&[remote.clone()]).unwrap(), 0);
        assert_eq!(db.get_note(&n.id).unwrap().title, "local");

        remote.title = "newer remote".into();
        remote.updated_at = local.updated_at + 1000;
        assert_eq!(db.apply_remote_notes(&[remote.clone()]).unwrap(), 1);
        assert_eq!(db.get_note(&n.id).unwrap().title, "newer remote");
        assert!(db.dirty_notes().unwrap().is_empty());

        // Brand-new rows from another device are inserted.
        remote.id = "other-device".into();
        assert_eq!(db.apply_remote_notes(&[remote]).unwrap(), 1);
        let hits = db
            .list_notes(&NoteFilter { query: Some("newer".into()), ..Default::default() })
            .unwrap();
        assert_eq!(hits.len(), 2, "pulled notes are searchable");
    }

    #[test]
    fn meta_roundtrip() {
        let db = db();
        assert_eq!(db.get_meta("k").unwrap(), None);
        db.set_meta("k", "1").unwrap();
        db.set_meta("k", "2").unwrap();
        assert_eq!(db.get_meta("k").unwrap().as_deref(), Some("2"));
        db.delete_meta("k").unwrap();
        assert_eq!(db.get_meta("k").unwrap(), None);
    }

    #[test]
    fn snippet_is_trimmed() {
        assert_eq!(collapse_whitespace("a\n\n b   c", 10), "a b c");
        assert_eq!(collapse_whitespace("ăâêôơư", 3), "ăâê…");
        assert_eq!(
            collapse_whitespace("## Việc\n- [ ] Họp **nhóm**\n> trích `code`", 80),
            "Việc Họp nhóm trích code"
        );
    }
}
