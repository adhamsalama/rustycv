use sqlx::SqlitePool;

use crate::ratelimit::RateLimiter;
use crate::render::Renderer;

/// Where the editor compiles PDFs.
///
/// Whoever runs the instance decides, once, at startup — the editor is told
/// and has no say. The two produce identical bytes (`rustycv-wasm` is a shim
/// over the very `render_pdf` this server calls), so this is a question about
/// which machine spends the CPU, which is the operator's to answer and not a
/// preference worth putting in front of someone writing a CV.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
// Renames the *variants* — which is what is wanted here, unlike the case in
// `rustycv-core` where it had to be `rename_all_fields`.
#[serde(rename_all = "camelCase")]
pub enum RenderMode {
    /// In the editor's own tab, on the wasm build of the renderer.
    ///
    /// The default, and it costs the server nothing. An editor that cannot
    /// load the module — nobody built one, or the fetch failed — falls back to
    /// asking this server anyway, so it is safe to leave on everywhere.
    #[default]
    Browser,
    /// On this server, over HTTP, once per edit.
    Server,
}

impl RenderMode {
    /// Read the value of `RUSTYCV_RENDER`.
    ///
    /// Nothing is accepted quietly. `PORT` falling back to 8080 on a typo is
    /// the trap this is written to avoid: a misspelled mode that started the
    /// server anyway would look exactly like the setting being ignored, and
    /// the only symptom would be renders happening in the wrong place.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim() {
            "browser" => Ok(Self::Browser),
            "server" => Ok(Self::Server),
            other => Err(format!(
                "RUSTYCV_RENDER must be `browser` or `server`, not `{other}`"
            )),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub renderer: Renderer,
    /// Shared by every request: the counters are the whole point, so cloning
    /// the state must not clone them. `RateLimiter` is an `Arc` inside.
    pub limiter: RateLimiter,
    /// A public share link is read by whoever holds it, with no account to
    /// key on — so unlike `limiter`, both of these are needed: one caps a
    /// single visitor hammering a link, the other caps a single link being
    /// hammered from many addresses at once (a link posted somewhere very
    /// public, or scraped from many IPs).
    pub public_ip_limiter: RateLimiter,
    pub public_cv_limiter: RateLimiter,
    /// Served to the editor over `/api/config`. Fixed for the life of the
    /// process, which is why the editor may cache it and never re-read it.
    pub render_mode: RenderMode,
}

#[cfg(test)]
mod tests {
    use super::RenderMode;

    #[test]
    fn an_unset_render_mode_is_the_browser() {
        assert_eq!(RenderMode::default(), RenderMode::Browser);
    }

    #[test]
    fn a_misspelled_render_mode_is_refused_rather_than_ignored() {
        assert_eq!(RenderMode::parse("browser"), Ok(RenderMode::Browser));
        assert_eq!(RenderMode::parse("server"), Ok(RenderMode::Server));
        // Whitespace is a copy-paste artefact, not a different answer.
        assert_eq!(RenderMode::parse("  server\n"), Ok(RenderMode::Server));

        for wrong in ["Browser", "client", "wasm", "true", ""] {
            assert!(
                RenderMode::parse(wrong).is_err(),
                "`{wrong}` should not have been accepted"
            );
        }
    }
}
