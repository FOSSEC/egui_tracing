use std::sync::{Arc, Mutex};

use tracing::{Event, Level, Subscriber};
#[cfg(feature = "log")]
use tracing_log::NormalizeEvent;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use super::event::CollectedEvent;

#[derive(Clone, Debug)]
pub enum AllowedTargets {
    All,
    Selected(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct EventCollector {
    allowed_targets: AllowedTargets,
    level: Level,
    events: Arc<Mutex<Vec<CollectedEvent>>>,
    max_events: Option<usize>,
}

impl EventCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_level(self, level: Level) -> Self {
        Self { level, ..self }
    }

    pub fn with_max_events(self, max_events: usize) -> Self {
        Self { max_events: Some(max_events), ..self }
    }

    pub fn allowed_targets(self, allowed_targets: AllowedTargets) -> Self {
        Self {
            allowed_targets,
            ..self
        }
    }

    pub fn events(&self) -> Vec<CollectedEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        *events = Vec::new();
    }

    fn collect(&self, event: CollectedEvent) {
        if event.level <= self.level {
            let should_collect = match self.allowed_targets {
                AllowedTargets::All => true,
                AllowedTargets::Selected(ref selection) => selection
                    .iter()
                    .any(|target| event.target.starts_with(target)),
            };
            if should_collect {
                let mut events = self.events.lock().unwrap();
                if let Some(max_events) = self.max_events {
                    if events.len() >= max_events {
                        events.remove(0);
                    }
                }
                events.push(event);
            }
        }
    }
}

impl Default for EventCollector {
    fn default() -> Self {
        Self {
            allowed_targets: AllowedTargets::All,
            events: Arc::new(Mutex::new(Vec::new())),
            level: Level::TRACE, // capture everything by default.
            max_events: None,
        }
    }
}

impl<S> Layer<S> for EventCollector
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        #[cfg(feature = "log")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "log")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "log"))]
        let meta = event.metadata();

        self.collect(CollectedEvent::new(event, meta));
    }
}
