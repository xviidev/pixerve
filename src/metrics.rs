use std::{
    fmt::Write,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

pub const BOUNDS: [f64; 11] = [
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

#[derive(Default)]
pub struct Metrics {
    pub requests: AtomicU64,
    pub redirects: AtomicU64,
    pub client_errors: AtomicU64,
    pub server_errors: AtomicU64,
    pub response_bytes: AtomicU64,
    pub ram_hit_bytes: AtomicU64,
    pub disk_hit_bytes: AtomicU64,
    pub shared_bytes: AtomicU64,
    pub miss_bytes: AtomicU64,
    pub ram_hits: AtomicU64,
    pub disk_hits: AtomicU64,
    pub shared: AtomicU64,
    pub misses: AtomicU64,
    pub source_reads: AtomicU64,
    pub source_bytes: AtomicU64,
    pub transforms: AtomicU64,
    pub transform_errors: AtomicU64,
    pub source_errors: AtomicU64,
    pub elapsed_us: AtomicU64,
    pub buckets: [AtomicU64; 11],
}

pub fn inc(counter: &AtomicU64) {
    counter.fetch_add(1, Ordering::Relaxed);
}
pub fn add(counter: &AtomicU64, n: u64) {
    counter.fetch_add(n, Ordering::Relaxed);
}

impl Metrics {
    pub fn observe(&self, status: u16, bytes: u64, elapsed: Duration, cache_status: Option<&str>) {
        inc(&self.requests);
        if status == 308 {
            inc(&self.redirects);
        }
        match cache_status {
            Some("RAM") => add(&self.ram_hit_bytes, bytes),
            Some("DISK") => add(&self.disk_hit_bytes, bytes),
            Some("SHARED") => add(&self.shared_bytes, bytes),
            Some("MISS") => add(&self.miss_bytes, bytes),
            _ => {}
        }
        if (400..500).contains(&status) {
            inc(&self.client_errors);
        }
        if status >= 500 {
            inc(&self.server_errors);
        }
        add(&self.response_bytes, bytes);
        add(
            &self.elapsed_us,
            elapsed.as_micros().min(u64::MAX as u128) as u64,
        );
        for (bound, bucket) in BOUNDS.iter().zip(&self.buckets) {
            if elapsed.as_secs_f64() <= *bound {
                inc(bucket);
            }
        }
    }

    pub fn render(&self, ram_bytes: u64, entries: u64, active: usize, queued: usize) -> String {
        let mut out = String::new();
        for (name, counter) in [
            ("requests_total", &self.requests),
            ("redirects_total", &self.redirects),
            ("client_errors_total", &self.client_errors),
            ("server_errors_total", &self.server_errors),
            ("response_bytes_total", &self.response_bytes),
            ("ram_hit_bytes_total", &self.ram_hit_bytes),
            ("disk_hit_bytes_total", &self.disk_hit_bytes),
            ("shared_bytes_total", &self.shared_bytes),
            ("miss_bytes_total", &self.miss_bytes),
            ("ram_hits_total", &self.ram_hits),
            ("disk_hits_total", &self.disk_hits),
            ("shared_responses_total", &self.shared),
            ("cache_misses_total", &self.misses),
            ("source_reads_total", &self.source_reads),
            ("source_bytes_total", &self.source_bytes),
            ("transforms_total", &self.transforms),
            ("transform_errors_total", &self.transform_errors),
            ("source_errors_total", &self.source_errors),
        ] {
            writeln!(
                out,
                "# TYPE img_{name} counter\nimg_{name} {}",
                counter.load(Ordering::Relaxed)
            )
            .unwrap();
        }
        writeln!(out, "# TYPE img_request_duration_seconds histogram").unwrap();
        for (bound, bucket) in BOUNDS.iter().zip(&self.buckets) {
            writeln!(
                out,
                "img_request_duration_seconds_bucket{{le=\"{bound}\"}} {}",
                bucket.load(Ordering::Relaxed)
            )
            .unwrap();
        }
        writeln!(out,"img_request_duration_seconds_bucket{{le=\"+Inf\"}} {}\nimg_request_duration_seconds_count {}\nimg_request_duration_seconds_sum {}",self.requests.load(Ordering::Relaxed),self.requests.load(Ordering::Relaxed),self.elapsed_us.load(Ordering::Relaxed) as f64/1_000_000.0).unwrap();
        for (name, value) in [
            ("ram_cache_bytes", ram_bytes),
            ("ram_cache_entries", entries),
            ("active_jobs", active as u64),
            ("queued_jobs", queued as u64),
        ] {
            writeln!(out, "# TYPE img_{name} gauge\nimg_{name} {value}").unwrap();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_bytes_errors_and_cumulative_latency_without_url_labels() {
        let stats = Metrics::default();
        stats.observe(200, 100, Duration::from_millis(20), Some("RAM"));
        stats.observe(503, 0, Duration::from_secs(1), None);
        let text = stats.render(1024, 1, 1, 2);
        assert!(text.contains("img_requests_total 2\n"));
        assert!(text.contains("img_ram_hit_bytes_total 100\n"));
        assert!(text.contains("img_server_errors_total 1\n"));
        assert!(text.contains("img_request_duration_seconds_bucket{le=\"0.025\"} 1\n"));
        assert!(text.contains("img_request_duration_seconds_bucket{le=\"1\"} 2\n"));
    }
}
