//! The page fetches of the loop (`openspec/specs/sensor-pages`,
//! docs/adr/0021).

use std::sync::mpsc;

use crate::{Fetch, Job};

/// Sends each job to the `fetch` thread, and writes each warning to standard
/// error.
#[derive(Debug)]
pub struct ChannelFetch {
    jobs: mpsc::Sender<Job>,
}

impl ChannelFetch {
    /// A fetch that sends its jobs on `jobs`.
    #[must_use]
    pub fn new(jobs: mpsc::Sender<Job>) -> Self {
        Self { jobs }
    }
}

impl Fetch for ChannelFetch {
    fn start(&mut self, job: Job) {
        let page = job.page;
        if self.jobs.send(job).is_err() {
            eprintln!(
                "gameclockd: the page '{}' has no data: the fetch thread stopped.",
                page.name()
            );
        }
    }

    fn warn(&mut self, line: &str) {
        eprintln!("{line}");
    }
}
