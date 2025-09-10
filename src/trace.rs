use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, Response},
};
use tower_http::{
    classify::{
        ClassifiedResponse, ClassifyResponse, NeverClassifyEos, SharedClassifier, StatusInRangeAsFailures, StatusInRangeFailureClass,
    },
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, MakeSpan as _, OnRequest, OnResponse, TraceLayer},
};
use tracing::Span;

pub(super) fn create_filtered_trace_layer()
-> TraceLayer<SharedClassifier<FilteredStatusAsFailures>, impl Fn(&Request<Body>) -> Span + Clone, FilterOnRequest, FilterOnResponse> {
    TraceLayer::new(
        FilteredStatusAsFailures {
            inner: StatusInRangeAsFailures::new(400..=599),
        }
        .into_make_classifier(),
    )
    .make_span_with(|req: &Request<Body>| {
        let path = req.uri().path();
        if path.starts_with("/tower-livereload") || path.starts_with("/.well-known/appspecific/com.chrome.devtools.json") {
            Span::none()
        } else {
            DefaultMakeSpan::default().make_span(req)
        }
    })
    .on_request(FilterOnRequest)
    .on_response(FilterOnResponse)
}

#[derive(Clone)]
pub(super) struct FilterOnRequest;
impl<B> OnRequest<B> for FilterOnRequest {
    fn on_request(&mut self, request: &Request<B>, span: &Span) {
        // if the span is the "none" span, it has no id — treat that as filtered
        if span.id().is_none() {
            return;
        }

        // otherwise, fall back to the default behavior
        DefaultOnRequest::default().on_request(request, span);
    }
}

#[derive(Clone)]
pub(super) struct FilterOnResponse;
impl<B> OnResponse<B> for FilterOnResponse {
    fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
        // if the span is the "none" span, it has no id — treat that as filtered
        if span.id().is_none() {
            return;
        }

        // otherwise, fall back to the default behavior
        DefaultOnResponse::default().on_response(response, latency, span);
    }
}
#[derive(Clone)]
pub(super) struct FilteredStatusAsFailures {
    inner: StatusInRangeAsFailures,
}

impl ClassifyResponse for FilteredStatusAsFailures {
    type ClassifyEos = NeverClassifyEos<Self::FailureClass>;
    type FailureClass = StatusInRangeFailureClass;

    fn classify_response<B>(self, res: &Response<B>) -> ClassifiedResponse<Self::FailureClass, Self::ClassifyEos> {
        let span = tracing::Span::current();
        if span.id().is_none() {
            // treat filtered paths as non-error
            ClassifiedResponse::Ready(Ok(()))
        } else {
            self.inner.classify_response(res)
        }
    }

    fn classify_error<E>(self, error: &E) -> Self::FailureClass
    where
        E: std::fmt::Display + 'static,
    {
        self.inner.classify_error(error)
    }
}

impl FilteredStatusAsFailures {
    fn into_make_classifier(self) -> SharedClassifier<Self> {
        SharedClassifier::new(self)
    }
}
