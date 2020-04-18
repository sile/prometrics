//! Metric label.
//!
//! # References
//!
//! - [Data model](https://prometheus.io/docs/concepts/data_model/)
//! - [Metric and label naming](https://prometheus.io/docs/practices/naming/)
use atomic_immut::AtomicImmut;
use std;
use std::fmt;
use std::ops::Deref;

use {ErrorKind, Result};

/// Metric label.
///
/// A label is a key-value pair.
///
/// Label names may contain ASCII letters, numbers, as well as underscores.
/// They must match the regex `[a-zA-Z_][a-zA-Z0-9_]*`.
/// Label names beginning with `__` are reserved for internal use.
///
/// Label values may contain any Unicode (utf-8) characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label {
    name: String,
    value: String,
}
impl Label {
    /// Makes a new `Label` instance.
    ///
    /// # Errors
    ///
    /// If `name` contains invalid characters, this function returns `ErrorKind::InvalidInput` error.
    ///
    /// # Examples
    ///
    /// ```
    /// use prometrics::ErrorKind;
    /// use prometrics::label::Label;
    ///
    /// let label = Label::new("foo", "bar").unwrap();
    /// assert_eq!(label.name(), "foo");
    /// assert_eq!(label.value(), "bar");
    /// assert_eq!(label.to_string(), r#"foo="bar""#);
    ///
    /// // Reserved name
    /// assert_eq!(Label::new("__foo", "bar").err().map(|e| *e.kind()),
    ///            Some(ErrorKind::InvalidInput));
    //
    /// // Invalid name
    /// assert_eq!(Label::new("fo-o", "bar").err().map(|e| *e.kind()),
    ///            Some(ErrorKind::InvalidInput));
    /// ```
    pub fn new(name: &str, value: &str) -> Result<Self> {
        track!(
            Self::validate_name(name),
            "name={:?}, value={:?}",
            name,
            value
        )?;
        Ok(Label {
            name: name.to_string(),
            value: value.to_string(),
        })
    }

    /// Returns the name of this label.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the value of this label.
    pub fn value(&self) -> &str {
        &self.value
    }

    fn validate_name(name: &str) -> Result<()> {
        // REGEX: [a-zA-Z_][a-zA-Z0-9_]*
        track_assert!(!name.is_empty(), ErrorKind::InvalidInput);
        track_assert!(!name.starts_with("__"), ErrorKind::InvalidInput, "Reserved");
        match name.as_bytes()[0] as char {
            'a'..='z' | 'A'..='Z' | '_' => {}
            _ => track_panic!(ErrorKind::InvalidInput),
        }
        for c in name.chars().skip(1) {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => {}
                _ => track_panic!(ErrorKind::InvalidInput),
            }
        }
        Ok(())
    }
}
impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // > `label_value` can be any sequence of UTF-8 characters,
        // > but the backslash, the double-quote, and the line-feed
        // > characters have to be escaped as `\\`, `\"`, and `\n`, respectively.
        write!(f, "{}=\"", self.name)?;
        for c in self.value.chars() {
            match c {
                '\\' => write!(f, "\\\\")?,
                '\n' => write!(f, "\\\\n")?,
                '"' => write!(f, "\\\"")?,
                _ => write!(f, "{}", c)?,
            }
        }
        write!(f, "\"")
    }
}

/// A map of labels (i.e., key-value pairs).
#[derive(Debug)]
pub struct Labels(AtomicImmut<Vec<Label>>);
impl Labels {
    /// Returns the number of labels contained in this map.
    pub fn len(&self) -> usize {
        self.0.load().len()
    }

    /// Returns `true` if this map has no labels, otherwise `false`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the label which has the name `name`.
    pub fn get(&self, name: &str) -> Option<&Label> {
        self.iter().find(|l| l.name() == name)
    }

    /// Returns an iterator which visiting all labels in this map.
    pub fn iter(&self) -> Iter {
        let labels = self.0.load();
        let inner = unsafe { std::mem::transmute(labels.iter()) };
        Iter { labels, inner }
    }

    pub(crate) fn new(labels: Vec<Label>) -> Self {
        Labels(AtomicImmut::new(labels))
    }
}
impl fmt::Display for Labels {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        for (i, label) in self.iter().enumerate() {
            if i != 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", label)?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

/// A mutable map of labels (i.e., key-value pairs).
#[derive(Debug)]
pub struct LabelsMut<'a> {
    inner: &'a Labels,
    reserved: Option<&'static str>,
}
impl<'a> LabelsMut<'a> {
    /// Inserts the label.
    pub fn insert(&mut self, name: &str, value: &str) -> Result<()> {
        track_assert_ne!(
            self.reserved.map(|s| &*s),
            Some(name),
            ErrorKind::InvalidInput
        );
        let label = track!(Label::new(name, value))?;
        self.inner.0.update(move |labels| {
            let mut labels = labels.clone();
            labels.retain(|l| l.name != label.name);
            labels.push(label.clone());
            labels.sort();
            labels
        });
        Ok(())
    }

    /// Removes the label which has the name `name` if it exists.
    pub fn remove(&mut self, name: &str) {
        self.inner
            .0
            .update(|labels| labels.iter().filter(|l| l.name != name).cloned().collect());
    }

    /// Clears the all labels.
    pub fn clear(&mut self) {
        self.inner.0.store(Vec::new());
    }

    pub(crate) fn new(labels: &'a Labels, reserved: Option<&'static str>) -> Self {
        LabelsMut {
            inner: labels,
            reserved,
        }
    }
}
impl<'a> Deref for LabelsMut<'a> {
    type Target = Labels;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

/// An iterator over the labels of a `Labels`.
#[derive(Debug)]
pub struct Iter<'a> {
    labels: std::sync::Arc<Vec<Label>>,
    inner: std::slice::Iter<'a, Label>,
}
impl<'a> Iterator for Iter<'a> {
    type Item = &'a Label;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
