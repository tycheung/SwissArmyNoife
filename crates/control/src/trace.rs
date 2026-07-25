//! Tracing helpers correlated with invoke / binding ids.

use tracing::Span;
use types::{BindingId, InvokeId};

/// Create an info-level span for a single invoke, tagged for correlation.
#[must_use]
pub fn invoke_span(invoke_id: InvokeId, binding_id: BindingId) -> Span {
    tracing::info_span!(
        "sak.invoke",
        invoke_id = %invoke_id,
        binding_id = %binding_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::Subscriber;
    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
    use tracing_subscriber::Registry;
    use uuid::Uuid;

    #[derive(Default)]
    struct FieldCapture {
        values: Mutex<Vec<(String, String)>>,
    }

    impl FieldCapture {
        fn snapshot(&self) -> Vec<(String, String)> {
            self.values.lock().expect("lock").clone()
        }
    }

    struct CapturingLayer {
        capture: Arc<FieldCapture>,
    }

    struct Recorder<'a> {
        out: &'a mut Vec<(String, String)>,
    }

    impl Visit for Recorder<'_> {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.out
                .push((field.name().to_owned(), format!("{value:?}")));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.out.push((field.name().to_owned(), value.to_owned()));
        }
    }

    impl<S> Layer<S> for CapturingLayer
    where
        S: Subscriber,
    {
        fn on_new_span(
            &self,
            attrs: &tracing::span::Attributes<'_>,
            _id: &tracing::span::Id,
            _ctx: Context<'_, S>,
        ) {
            let mut recorded = Vec::new();
            let mut visitor = Recorder { out: &mut recorded };
            attrs.record(&mut visitor);
            self.capture.values.lock().expect("lock").extend(recorded);
        }
    }

    #[test]
    fn invoke_span_records_correlation_fields() {
        let capture = Arc::new(FieldCapture::default());
        let layer = CapturingLayer {
            capture: Arc::clone(&capture),
        };
        let subscriber = Registry::default().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            let invoke_id = InvokeId::from_uuid(Uuid::nil());
            let binding_id = BindingId::from_uuid(Uuid::from_u128(1));
            let span = invoke_span(invoke_id, binding_id);
            let _entered = span.entered();
            tracing::info!("invoke started");
        });

        let fields = capture.snapshot();
        let invoke = fields
            .iter()
            .find(|(k, _)| k == "invoke_id")
            .map(|(_, v)| v.as_str());
        let binding = fields
            .iter()
            .find(|(k, _)| k == "binding_id")
            .map(|(_, v)| v.as_str());

        assert_eq!(
            invoke,
            Some("00000000-0000-0000-0000-000000000000"),
            "fields={fields:?}"
        );
        assert_eq!(
            binding,
            Some("00000000-0000-0000-0000-000000000001"),
            "fields={fields:?}"
        );
    }
}
