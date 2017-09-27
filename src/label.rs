use std;
use atomic_immut::AtomicImmut;

use {Result, ErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label {
    name: String,
    value: String,
}
impl Label {
    pub fn new(name: &str, value: &str) -> Result<Self> {
        track!(
            validate_label_name(name),
            "name={:?}, value={:?}",
            name,
            value
        )?;
        Ok(Label {
            name: name.to_string(),
            value: value.to_string(),
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn value(&self) -> &str {
        &self.value
    }
}

fn validate_label_name(name: &str) -> Result<()> {
    // REGEX: [a-zA-Z_][a-zA-Z0-9_]*
    track_assert!(!name.is_empty(), ErrorKind::InvalidInput);
    track_assert!(!name.starts_with("__"), ErrorKind::InvalidInput, "Reserved");
    match name.as_bytes()[0] as char {
        'a'...'z' | 'A'...'Z' | '_' => {}
        _ => track_panic!(ErrorKind::InvalidInput),
    }
    for c in name.chars().skip(1) {
        match c {
            'a'...'z' | 'A'...'Z' | '0'...'9' | '_' => {}
            _ => track_panic!(ErrorKind::InvalidInput),
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct Labels(AtomicImmut<Vec<Label>>);
impl Labels {
    pub(crate) fn new(labels: Vec<Label>) -> Self {
        Labels(AtomicImmut::new(labels))
    }
    pub fn iter(&self) -> Iter {
        let labels = self.0.load();
        Iter {
            labels: labels.clone(),
            inner: unsafe { std::mem::transmute(labels.iter()) },
        }
    }
    pub fn len(&self) -> usize {
        self.0.load().len()
    }
}

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

#[derive(Debug)]
pub struct LabelsMut<'a>(&'a AtomicImmut<Vec<Label>>);
impl<'a> LabelsMut<'a> {
    pub(crate) fn new(labels: &'a Labels) -> Self {
        LabelsMut(&labels.0)
    }
    pub fn clear(&mut self) -> &mut Self {
        self.0.store(Vec::new());
        self
    }
    pub fn remove(&mut self, name: &str) -> &mut Self {
        self.0.update(|labels| {
            labels.iter().filter(|l| l.name != name).cloned().collect()
        });
        self
    }
    pub fn insert(&mut self, name: &str, value: &str) -> Result<&mut Self> {
        let new = track!(Label::new(name, value))?;
        self.0.update(move |labels| {
            let mut labels = labels.clone();
            labels.retain(|l| l.name != name);
            labels.push(new.clone());
            labels.sort();
            labels
        });
        Ok(self)
    }
    pub fn update(&mut self, name: &str, value: &str) -> &mut Self {
        let new = Label {
            name: name.to_string(),
            value: value.to_string(),
        };
        self.0.update(move |labels| {
            labels
                .iter()
                .map(|l| if l.name == name {
                    new.clone()
                } else {
                    l.clone()
                })
                .collect()
        });
        self
    }
    pub fn len(&self) -> usize {
        self.0.load().len()
    }
}
