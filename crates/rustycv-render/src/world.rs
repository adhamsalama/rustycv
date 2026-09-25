//! A [`World`] that serves a single CV render entirely from memory.
//!
//! There is no filesystem and no package resolution: the only files that exist
//! are the template sources we embedded at build time plus a synthetic
//! `/data.json` holding the CV being rendered. That is what makes running Typst
//! here safe even though templates are code — a template cannot reach anything
//! we did not hand it.

use std::collections::HashMap;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use crate::fonts;

/// Build a [`FileId`] for an absolute in-project path such as `/main.typ`.
pub fn file_id(path: &str) -> FileId {
    let vpath = VirtualPath::new(path).expect("template paths are valid virtual paths");
    FileId::new(RootedPath::new(VirtualRoot::Project, vpath))
}

pub struct CvWorld {
    library: LazyHash<Library>,
    book: &'static LazyHash<FontBook>,
    fonts: &'static [Font],
    main: FileId,
    /// Typst source files, keyed by id. Parsed once per world.
    sources: HashMap<FileId, Source>,
    /// Non-Typst files — currently just the CV JSON.
    files: HashMap<FileId, Bytes>,
    today: Option<Datetime>,
}

impl CvWorld {
    /// `main` is the template's entry source, `extra_sources` are its imports,
    /// and `assets` are non-Typst files (icons) that templates `read()`.
    pub fn new(
        main_source: &str,
        extra_sources: &[(&str, &str)],
        assets: &[(&str, &'static [u8])],
        data_json: String,
        today: Option<Datetime>,
    ) -> Self {
        let main = file_id("/main.typ");

        let mut sources = HashMap::new();
        sources.insert(main, Source::new(main, main_source.to_string()));
        for (path, text) in extra_sources {
            let id = file_id(path);
            sources.insert(id, Source::new(id, (*text).to_string()));
        }

        let mut files = HashMap::new();
        files.insert(file_id("/data.json"), Bytes::new(data_json.into_bytes()));
        for (path, bytes) in assets {
            files.insert(file_id(path), Bytes::new(*bytes));
        }

        let shared = fonts::shared();
        Self {
            library: LazyHash::new(Library::default()),
            book: &shared.book,
            fonts: &shared.fonts,
            main,
            sources,
            files,
            today,
        }
    }

    fn not_found(id: FileId) -> FileError {
        FileError::NotFound(std::path::PathBuf::from(id.get().vpath().get_with_slash()))
    }
}

impl World for CvWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        // `Source` is an `Arc` inside, so this clone is a refcount bump.
        self.sources
            .get(&id)
            .cloned()
            .ok_or_else(|| Self::not_found(id))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| Self::not_found(id))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        // Fixed per render. Templates do not print dates today, but pinning this
        // keeps renders byte-reproducible for the same input.
        self.today
    }
}
