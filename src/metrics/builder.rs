use std::time::Duration;

use {default_registry, Registry};
use metrics::{CounterBuilder, ObservedCounterBuilder, GaugeBuilder, HistogramBuilder, SummaryBuilder};

/// Common builder for various metrics.
#[derive(Debug, Clone)]
pub struct MetricBuilder {
    namespace: Option<String>,
    subsystem: Option<String>,
    labels: Vec<(String, String)>,
    registries: Vec<Registry>,
}
impl MetricBuilder {
    /// Makes a builder with the default registry.
    pub fn new() -> Self {
        Self::with_registry(default_registry())
    }

    /// Makes a builder with the given registry.
    pub fn with_registry(registry: Registry) -> Self {
        MetricBuilder {
            namespace: None,
            subsystem: None,
            labels: Vec::new(),
            registries: vec![registry],
        }
    }

    /// Makes a builder without any registries.
    pub fn without_registry() -> Self {
        MetricBuilder {
            namespace: None,
            subsystem: None,
            labels: Vec::new(),
            registries: Vec::new(),
        }
    }

    /// Sets the namespace part of the metric name.
    pub fn namespace(&mut self, namespace: &str) -> &mut Self {
        self.namespace = Some(namespace.to_owned());
        self
    }

    /// Sets the subsystem part of the metric name of this.
    pub fn subsystem(&mut self, subsystem: &str) -> &mut Self {
        self.subsystem = Some(subsystem.to_owned());
        self
    }

    /// Adds a label.
    ///
    /// Note that `name` will be validated when creating the metrics.
    pub fn label(&mut self, name: &str, value: &str) -> &mut Self {
        self.labels.push((name.to_owned(), value.to_owned()));
        self
    }

    /// Adds a registry to which the resulting metrics will be registered.
    pub fn registry(&mut self, registry: Registry) -> &mut Self {
        self.registries.push(registry);
        self
    }

    /// Clears the current registries, then sets `registry` as the new one.
    pub fn set_registry(&mut self, registry: Registry) -> &mut Self {
        self.registries = vec![registry];
        self
    }

    /// Makes a `CounterBuilder` that inherited the setting of this builder.
    pub fn counter(&self, name: &str) -> CounterBuilder {
        let mut builder = CounterBuilder::new(name);
        if let Some(ref namespace) = self.namespace {
            builder.namespace(namespace);
        }
        if let Some(ref subsystem) = self.subsystem {
            builder.subsystem(subsystem);
        }
        for &(ref k, ref v) in &self.labels {
            builder.label(k, v);
        }
        for r in &self.registries {
            builder.registry(r.clone());
        }
        builder
    }

    /// Makes a `ObservedCounterBuilder` that inherited the setting of this builder.
    pub fn observed_counter(&self, name: &str) -> ObservedCounterBuilder {
        let mut builder = ObservedCounterBuilder::new(name);
        if let Some(ref namespace) = self.namespace {
            builder.namespace(namespace);
        }
        if let Some(ref subsystem) = self.subsystem {
            builder.subsystem(subsystem);
        }
        for &(ref k, ref v) in &self.labels {
            builder.label(k, v);
        }
        for r in &self.registries {
            builder.registry(r.clone());
        }
        builder
    }

    /// Makes a `GaugeBuilder` that inherited the setting of this builder.
    pub fn gauge(&self, name: &str) -> GaugeBuilder {
        let mut builder = GaugeBuilder::new(name);
        if let Some(ref namespace) = self.namespace {
            builder.namespace(namespace);
        }
        if let Some(ref subsystem) = self.subsystem {
            builder.subsystem(subsystem);
        }
        for &(ref k, ref v) in &self.labels {
            builder.label(k, v);
        }
        for r in &self.registries {
            builder.registry(r.clone());
        }
        builder
    }

    /// Makes a `HistogramBuilder` that inherited the setting of this builder.
    pub fn histogram(&self, name: &str) -> HistogramBuilder {
        let mut builder = HistogramBuilder::new(name);
        if let Some(ref namespace) = self.namespace {
            builder.namespace(namespace);
        }
        if let Some(ref subsystem) = self.subsystem {
            builder.subsystem(subsystem);
        }
        for &(ref k, ref v) in &self.labels {
            builder.label(k, v);
        }
        for r in &self.registries {
            builder.registry(r.clone());
        }
        builder
    }

    /// Makes a `SummaryBuilder` that inherited the setting of this builder.
    pub fn summary(&self, name: &str, window: Duration) -> SummaryBuilder {
        let mut builder = SummaryBuilder::new(name, window);
        if let Some(ref namespace) = self.namespace {
            builder.namespace(namespace);
        }
        if let Some(ref subsystem) = self.subsystem {
            builder.subsystem(subsystem);
        }
        for &(ref k, ref v) in &self.labels {
            builder.label(k, v);
        }
        for r in &self.registries {
            builder.registry(r.clone());
        }
        builder
    }
}
impl Default for MetricBuilder {
    fn default() -> Self {
        Self::new()
    }
}
