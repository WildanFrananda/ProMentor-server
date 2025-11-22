use std::time::Instant;

use actix_web::{HttpResponse, Error};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::body::BoxBody;
use lazy_static::lazy_static;
use prometheus::{CounterVec, Encoder, HistogramVec, Opts, HistogramOpts, TextEncoder};

lazy_static! {
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("http_requests_total", "Total number of HTTP requests"),
        &["method", "path", "status_code"]
    )
    .expect("Failed to create counter metric");

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration in seconds"),
        &["method", "path", "status_code"]
    )
    .expect("Failed to create histogram metric");
}

pub fn register_metrics() {
    let registry = prometheus::default_registry();

    registry
        .register(Box::new(HTTP_REQUESTS_TOTAL.clone()))
        .expect("Failed to register counter metric");

    registry
        .register(Box::new(HTTP_REQUEST_DURATION.clone()))
        .expect("Failed to register histogram metric");

    println!("📊 Prometheus Metrics has registered.");
}

pub async fn metrics_handler() -> impl actix_web::Responder {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    // gather metrics from the default registry
    let metric_families = prometheus::gather();

    encoder.encode(&metric_families, &mut buffer).unwrap();

    HttpResponse::Ok().content_type(encoder.format_type()).body(buffer)
}

pub async fn metrics_middleware(
    req: ServiceRequest,
    next: Next<BoxBody>,
) -> Result<ServiceResponse, Error> {
    let start = Instant::now();
    let path = req.match_pattern().unwrap_or_else(|| req.path().to_string());
    let method = req.method().to_string();

    // call the next middleware / handler
    let res = next.call(req).await?;
    let status = res.status().as_u16().to_string();
    let duration = start.elapsed().as_secs_f64();

    HTTP_REQUESTS_TOTAL.with_label_values(&[&method, &path, &status]).inc();
    HTTP_REQUEST_DURATION.with_label_values(&[&method, &path, &status]).observe(duration);

    Ok(res)
}
