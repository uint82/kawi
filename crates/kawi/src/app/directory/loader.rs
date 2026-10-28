use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use ropey::Rope;

use super::listing::{listing_text, read_listing, Listing, SortMode};
use super::metadata::MetadataColumns;

pub(super) struct DirectoryLoader {
    requests: Sender<LoadRequest>,
    results: Receiver<LoadResult>,
    next_id: u64,
}

struct LoadRequest {
    id: u64,
    root: PathBuf,
    show_hidden: bool,
    sort: SortMode,
    columns: MetadataColumns,
    include_text: bool,
}

pub(super) struct LoadedListing {
    pub(super) listing: Listing,
    pub(super) text: Option<Rope>,
}

pub(super) struct LoadResult {
    pub(super) id: u64,
    pub(super) listing: Result<LoadedListing, String>,
}

impl DirectoryLoader {
    pub(super) fn new(wake: Sender<()>) -> Result<Self, String> {
        let (requests, request_rx) = mpsc::channel();
        let (result_tx, results) = mpsc::channel();
        thread::Builder::new()
            .name("kawi-directory".to_string())
            .spawn(move || run(request_rx, result_tx, wake))
            .map_err(|error| format!("cannot start directory loader: {error}"))?;
        Ok(Self {
            requests,
            results,
            next_id: 0,
        })
    }

    pub(super) fn request(
        &mut self,
        root: PathBuf,
        show_hidden: bool,
        sort: SortMode,
        columns: MetadataColumns,
        include_text: bool,
    ) -> Result<u64, String> {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        self.requests
            .send(LoadRequest {
                id,
                root,
                show_hidden,
                sort,
                columns,
                include_text,
            })
            .map_err(|_| "directory loader stopped".to_string())?;
        Ok(id)
    }

    pub(super) fn try_recv(&self) -> Option<LoadResult> {
        self.results.try_recv().ok()
    }
}

fn run(requests: Receiver<LoadRequest>, results: Sender<LoadResult>, wake: Sender<()>) {
    while let Ok(request) = requests.recv() {
        let listing = read_listing(
            &request.root,
            request.show_hidden,
            1,
            request.sort,
            request.columns,
        )
        .map(|listing| {
            let text = request
                .include_text
                .then(|| Rope::from_str(&listing_text(&listing)));
            LoadedListing { listing, text }
        })
        .map_err(|error| format!("Cannot read {}: {error}", request.root.display()));
        if results
            .send(LoadResult {
                id: request.id,
                listing,
            })
            .is_err()
        {
            return;
        }
        let _ = wake.send(());
    }
}
