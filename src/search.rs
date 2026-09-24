//! Full-text search using SQLite FTS5

use std::collections::HashSet;

use color_eyre::Result;
use rusqlite::{Connection, params};
use rust_apt::cache::PackageSort;

use crate::apt::AptCache;

/// SQLite FTS5 search index over package names and summaries
pub struct SearchIndex {
    conn: Connection,
}

impl SearchIndex {
    fn empty() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE VIRTUAL TABLE packages USING fts5(name, description)",
            [],
        )?;
        Ok(Self { conn })
    }

    /// Build the index from the APT cache
    pub fn build(apt: &AptCache) -> Result<Self> {
        let mut index = Self::empty()?;
        index.insert_all(apt.packages(&PackageSort::default()).map(|pkg| {
            let desc = pkg
                .candidate()
                .and_then(|c| c.summary())
                .unwrap_or_default();
            (pkg.name().to_string(), desc)
        }))?;
        Ok(index)
    }

    fn insert_all(&mut self, rows: impl Iterator<Item = (String, String)>) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare("INSERT INTO packages (name, description) VALUES (?, ?)")?;
            for (name, desc) in rows {
                stmt.execute(params![name, desc])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Names of packages matching every word of the query as a prefix
    pub fn search(&self, query: &str) -> Result<HashSet<String>> {
        let Some(fts_query) = fts_query(query) else {
            return Ok(HashSet::new());
        };
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT name FROM packages WHERE packages MATCH ?")?;
        let rows = stmt.query_map([&fts_query], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

/// Turn user input into an FTS5 query: each word becomes a quoted phrase
/// with prefix matching, so package-name punctuation (`-`, `.`, `+`) is
/// tokenized like the indexed text instead of parsed as query syntax.
fn fts_query(query: &str) -> Option<String> {
    let words: Vec<String> = query
        .split_whitespace()
        .map(|word| format!("\"{}\"*", word.replace('"', "\"\"")))
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> SearchIndex {
        let mut index = SearchIndex::empty().unwrap();
        index
            .insert_all(
                [
                    ("python3-foo", "Foo bindings"),
                    ("libc6.1", "C library"),
                    ("libstdc++6", "C++ runtime"),
                    ("vim", "Vi IMproved"),
                ]
                .into_iter()
                .map(|(n, d)| (n.to_string(), d.to_string())),
            )
            .unwrap();
        index
    }

    fn names(query: &str) -> Vec<String> {
        let mut v: Vec<String> = index().search(query).unwrap().into_iter().collect();
        v.sort();
        v
    }

    #[test]
    fn punctuation_in_package_names() {
        assert_eq!(names("python3-foo"), ["python3-foo"]);
        assert_eq!(names("python3-"), ["python3-foo"]);
        assert_eq!(names("libc6.1"), ["libc6.1"]);
        assert_eq!(names("libstdc++"), ["libstdc++6"]);
    }

    #[test]
    fn query_syntax_is_inert() {
        for q in ["\"", "*", "a:b", "(vim", "NOT vim", "-", "{x}"] {
            assert!(index().search(q).is_ok(), "query {q:?} errored");
        }
    }

    #[test]
    fn prefix_and_description_match() {
        assert_eq!(names("vi"), ["vim"]);
        assert_eq!(names("runtime"), ["libstdc++6"]);
        assert!(names("   ").is_empty());
    }
}
