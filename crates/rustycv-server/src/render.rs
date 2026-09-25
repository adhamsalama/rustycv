//! The render service.
//!
//! Typst compilation is synchronous and CPU-bound, so it runs on the blocking
//! pool behind a semaphore. Without the bound, a few editors typing at once
//! would each get a thread and the machine would spend more time context
//! switching than laying out pages.

use std::sync::Arc;

use rustycv_core::CvDocument;
use rustycv_render::RenderError;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct Renderer {
    permits: Arc<Semaphore>,
}

impl Renderer {
    pub fn new() -> Self {
        // Leave a core free so the HTTP side stays responsive while rendering.
        let parallelism = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(2);
        Self {
            permits: Arc::new(Semaphore::new(parallelism)),
        }
    }

    pub async fn pdf(&self, document: CvDocument) -> Result<Vec<u8>, RenderError> {
        self.run(move || rustycv_render::render_pdf(&document))
            .await
    }

    async fn run<T, F>(&self, f: F) -> Result<T, RenderError>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, RenderError> + Send + 'static,
    {
        let _permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .expect("the render semaphore is never closed");

        tokio::task::spawn_blocking(f)
            .await
            .expect("the render task does not panic")
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
