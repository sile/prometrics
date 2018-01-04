extern crate prometrics;
#[macro_use]
extern crate trackable;

use prometrics::metrics::ProcessMetricsCollector;
use trackable::error::Failure;

fn main() {
    track_try_unwrap!(prometrics::default_registry().register(ProcessMetricsCollector::new(),));

    let mut gatherer = track_try_unwrap!(
        prometrics::default_gatherer()
            .lock()
            .map_err(|e| Failure::from_error(e.to_string()))
    );
    for metric in gatherer.gather() {
        println!("{}", metric);
    }
}
