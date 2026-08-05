use askama::Template;
use axum::{extract::State, response::Html};

use crate::{
    domain::{Container, Sample},
    web::{AppState, error::AppError},
};

/// How many samples fit in a history strip. Anything older than this is simply
/// not requested from the store — `recent` returns the newest first, so the
/// oldest samples fall off the left edge.
const HISTORY_BARS: usize = 60;

/// Drawn width of one bar, in pixels.
const BAR_WIDTH: usize = 5;

/// Horizontal pitch of one bar: its width plus the gap to the next one. The
/// whole pitch is hoverable, not just the painted part.
const BAR_PITCH: usize = 7;

/// One bar in a history strip.
struct Bar {
    up: bool,
    /// Rendered into a `data-label` attribute and shown by a CSS `::after`
    /// tooltip on hover.
    label: String,
}

impl Bar {
    fn new(sample: &Sample) -> Self {
        let at = sample.at;
        Self {
            up: sample.state.is_running(),
            label: format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC — {}",
                at.year(),
                at.month() as u8,
                at.day(),
                at.hour(),
                at.minute(),
                at.second(),
                sample.state,
            ),
        }
    }
}

/// A container plus its recent uptime, oldest sample first.
struct Row {
    container: Container,
    history: Vec<Bar>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    rows: Vec<Row>,
    /// Fixed width of every strip, whether or not there's enough history to
    /// fill it.
    strip_width: usize,
    bar_width: usize,
    bar_gap: usize,
}

/// Live metadata comes from the runtime, history from the store. Keeping the two
/// separate means the page still renders before the poller has recorded
/// anything, and `Sample` stays small enough to keep thousands per container.
pub async fn index(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let containers = state.runtime.list().await?;

    let mut rows = Vec::with_capacity(containers.len());
    for container in containers {
        let mut samples = state.store.recent(&container.id, HISTORY_BARS).await?;

        // `recent` hands back newest-first; the strip reads left to right. A
        // partly filled strip is right-aligned by the flexbox, so the newest
        // sample always sits at the right edge.
        samples.reverse();

        let history = samples.iter().map(Bar::new).collect();
        rows.push(Row { container, history });
    }

    Ok(Html(
        IndexTemplate {
            rows,
            strip_width: HISTORY_BARS * BAR_PITCH,
            bar_width: BAR_WIDTH,
            bar_gap: BAR_PITCH - BAR_WIDTH,
        }
        .render()?,
    ))
}
