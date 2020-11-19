use crate::event::{
    lookup::{Segment, SegmentBuf},
    Lookup, LookupBuf, Value,
};
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use std::{
    borrow::Borrow,
    collections::{btree_map::Entry, BTreeMap, HashMap},
    convert::{TryFrom, TryInto},
    fmt::Debug,
    iter::FromIterator,
};

#[derive(PartialEq, Debug, Clone, Default)]
pub struct LogEvent {
    fields: BTreeMap<String, Value>,
}

impl LogEvent {
    /// Get an immutable borrow of the given value by lookup.
    #[instrument(level = "trace", skip(self))]
    pub fn get<'a>(&self, lookup: impl Borrow<Lookup<'a>> + Debug) -> Option<&Value> {
        let lookup = lookup.borrow();
        let mut lookup_iter = lookup.iter();
        // The first step should always be a field.
        let first_step = lookup_iter.next()?;
        // This is good, since the first step into a LogEvent will also be a field.

        // This step largely exists so that we can make `cursor` a `Value` right off the bat.
        // We couldn't go like `let cursor = Value::from(self.fields)` since that'd take the value.
        let mut cursor = match first_step {
            Segment::Field(ref f) => self.fields.get(*f),
            // In this case, the user has passed us an invariant.
            Segment::Index(_) => {
                error!(
                    "Lookups into LogEvents should never start with indexes.\
                        Please report your config."
                );
                return None;
            }
        };

        for segment in lookup_iter {
            // Don't do extra work.
            if cursor.is_none() {
                break;
            }
            cursor = match (segment, cursor) {
                // Fields access maps.
                (Segment::Field(ref f), Some(Value::Map(map))) => {
                    trace!("Matched field into map.");
                    map.get(*f)
                }
                // Indexes access arrays.
                (Segment::Index(i), Some(Value::Array(array))) => {
                    trace!("Matched index into array");
                    array.get(*i)
                }
                // The rest, it's not good.
                (Segment::Index(_), _) | (Segment::Field(_), _) => {
                    trace!("Unmatched lookup.");
                    None
                }
            }
        }

        // By the time we get here we either have the item, or nothing. Either case, we're correct.
        cursor
    }

    /// Get a mutable borrow of the value by lookup.
    #[instrument(level = "trace", skip(self))]
    pub fn get_mut<'a>(&mut self, lookup: impl Borrow<Lookup<'a>> + Debug) -> Option<&mut Value> {
        let lookup = lookup.borrow();
        let mut lookup_iter = lookup.iter();
        // The first step should always be a field.
        let first_step = lookup_iter.next()?;
        // This is good, since the first step into a LogEvent will also be a field.

        // This step largely exists so that we can make `cursor` a `Value` right off the bat.
        // We couldn't go like `let cursor = Value::from(self.fields)` since that'd take the value.
        let mut cursor = match first_step {
            Segment::Field(f) => self.fields.get_mut(*f),
            // In this case, the user has passed us an invariant.
            Segment::Index(_) => {
                error!(
                    "Lookups into LogEvents should never start with indexes.\
                        Please report your config.."
                );
                return None;
            }
        };

        for segment in lookup_iter {
            // Don't do extra work.
            if cursor.is_none() {
                break;
            }
            cursor = match (segment, cursor) {
                // Fields access maps.
                (Segment::Field(f), Some(Value::Map(map))) => {
                    trace!("Matched field into map.");
                    map.get_mut(*f)
                }
                // Indexes access arrays.
                (Segment::Index(i), Some(Value::Array(array))) => {
                    trace!("Matched index into array");
                    array.get_mut(*i)
                }
                // The rest, it's not good.
                (Segment::Index(_), _) | (Segment::Field(_), _) => {
                    trace!("Unmatched lookup.");
                    None
                }
            }
        }

        // By the time we get here we either have the item, or nothing. Either case, we're correct.
        cursor
    }

    /// Determine if the log event contains a value at a given lookup.
    #[instrument(level = "trace", skip(self))]
    pub fn contains<'a>(&self, lookup: impl Borrow<Lookup<'a>> + Debug) -> bool {
        self.get(lookup).is_some()
    }

    /// Insert a value at a given lookup.
    #[instrument(level = "trace", skip(self))]
    pub fn insert(&mut self, lookup: LookupBuf, value: impl Into<Value> + Debug) -> Option<Value> {
        unimplemented!()
    }

    /// Remove a value that exists at a given lookup.
    ///
    /// Setting `prune` to true will also remove the entries of maps and arrays that are emptied.
    #[instrument(level = "trace", skip(self))]
    pub fn remove<'a>(
        &mut self,
        lookup: impl Borrow<Lookup<'a>> + Debug,
        prune: bool,
    ) -> Option<Value> {
        unimplemented!()
    }

    /// Iterate over the lookups available in this log event.
    ///
    /// This is notably different than the keys in a map, as this descends into things like arrays
    /// and maps. It also returns those array/map values during iteration.
    #[instrument(level = "trace", skip(self))]
    pub fn keys<'a>(&'a self) -> impl Iterator<Item = Lookup<'a>> + 'a {
        unimplemented!();
    }

    /// Iterate over all lookup/value pairs.
    ///
    /// This is notably different than pairs in a map, as this descends into things like arrays and
    /// maps. It also returns those array/map values during iteration.
    #[instrument(level = "trace", skip(self))]
    pub fn all_fields<'a>(&'a self) -> impl Iterator<Item = (Lookup<'a>, &'a Value)> + Serialize {
        unimplemented!();
    }

    /// Determine if the log event is empty of fields.
    #[instrument(level = "trace", skip(self))]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Return an entry for the given lookup.
    #[instrument(level = "trace", skip(self, lookup), fields(lookup = %lookup), err)]
    fn entry(&mut self, lookup: LookupBuf) -> crate::Result<Entry<String, Value>> {
        trace!("Seeking to entry.");
        let mut walker = lookup.into_iter().enumerate();

        let mut current_pointer = if let Some((index, SegmentBuf::Field(segment))) = walker.next() {
            trace!(%segment, index, "Seeking segment.");
            self.fields.entry(segment)
        } else {
            unreachable!(
                "It is an invariant to have a `Lookup` without a contained `Segment`.\
                `Lookup::is_valid` should catch this during `Lookup` creation, maybe it was not \
                called?."
            );
        };

        for (index, segment) in walker {
            trace!(%segment, index, "Seeking next segment.");
            current_pointer = match (segment, current_pointer) {
                (SegmentBuf::Field(field), Entry::Occupied(entry)) => match entry.into_mut() {
                    Value::Map(map) => map.entry(field),
                    v => return Err(format!("Looking up field on a non-map value: {:?}", v).into()),
                },
                (SegmentBuf::Field(field), Entry::Vacant(entry)) => {
                    trace!(segment = %field, index, "Met vacant entry.");
                    return Err(format!(
                        "Tried to step into `{}` of `{}`, but it did not exist.",
                        field,
                        entry.key()
                    )
                    .into());
                }
                _ => return Err("The entry API cannot yet descend into array indices.".into()),
            };
        }
        trace!(entry = ?current_pointer, "Result.");
        Ok(current_pointer)
    }

    /// Returns the entire event as a `Value::Map`.
    #[instrument(level = "trace", skip(self))]
    pub fn take(self) -> Value {
        Value::Map(self.fields)
    }
}

impl From<BTreeMap<String, Value>> for LogEvent {
    fn from(map: BTreeMap<String, Value>) -> Self {
        LogEvent { fields: map }
    }
}

impl Into<BTreeMap<String, Value>> for LogEvent {
    fn into(self) -> BTreeMap<String, Value> {
        let Self { fields } = self;
        fields
    }
}

impl From<HashMap<String, Value>> for LogEvent {
    fn from(map: HashMap<String, Value>) -> Self {
        LogEvent {
            fields: map.into_iter().collect(),
        }
    }
}

impl Into<HashMap<String, Value>> for LogEvent {
    fn into(self) -> HashMap<String, Value> {
        self.fields.into_iter().collect()
    }
}

impl TryFrom<serde_json::Value> for LogEvent {
    type Error = crate::Error;

    fn try_from(map: serde_json::Value) -> Result<Self, Self::Error> {
        match map {
            serde_json::Value::Object(fields) => Ok(LogEvent::from(
                fields
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect::<BTreeMap<_, _>>(),
            )),
            _ => Err(crate::Error::from(
                "Attempted to convert non-Object JSON into a LogEvent.",
            )),
        }
    }
}

impl TryInto<serde_json::Value> for LogEvent {
    type Error = crate::Error;

    fn try_into(self) -> Result<serde_json::Value, Self::Error> {
        let Self { fields } = self;
        Ok(serde_json::to_value(fields)?)
    }
}

impl<'a, T> std::ops::Index<T> for LogEvent
where
    T: Borrow<Lookup<'a>>,
{
    type Output = Value;

    fn index(&self, key: T) -> &Value {
        let key = key.borrow();
        self.get(key)
            .expect(&*format!("Key is not found: {:?}", key))
    }
}

impl<'a, T> std::ops::IndexMut<T> for LogEvent
where
    T: Borrow<Lookup<'a>>,
{
    fn index_mut(&mut self, key: T) -> &mut Value {
        let key = key.borrow();
        self.get_mut(key)
            .expect(&*format!("Key is not found: {:?}", key))
    }
}

impl<'a, V> Extend<(LookupBuf, V)> for LogEvent
where
    V: Into<Value>,
{
    fn extend<I: IntoIterator<Item = (LookupBuf, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v.into());
        }
    }
}

// Allow converting any kind of appropriate key/value iterator directly into a LogEvent.
impl<'a, V: Into<Value>> FromIterator<(LookupBuf, V)> for LogEvent {
    fn from_iter<T: IntoIterator<Item = (LookupBuf, V)>>(iter: T) -> Self {
        let mut log_event = LogEvent::default();
        log_event.extend(iter);
        log_event
    }
}

/// Converts event into an iterator over top-level key/value pairs.
impl IntoIterator for LogEvent {
    type Item = (String, Value);
    type IntoIter = std::collections::btree_map::IntoIter<String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

impl Serialize for LogEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(self.fields.iter())
    }
}

impl<'de> Deserialize<'de> for LogEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error> where
        D: Deserializer<'de> {
        deserializer.deserialize_map(crate::event::util::LogEventVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_util::open_fixture;
    use serde_json::json;
    use tracing::trace;

    // This test iterates over the `tests/data/fixtures/log_event` folder and:
    //   * Ensures the EventLog parsed from bytes and turned into a serde_json::Value are equal to the
    //     item being just plain parsed as json.
    //
    // Basically: This test makes sure we aren't mutilating any content users might be sending.
    #[test]
    fn json_value_to_vector_log_event_to_json_value() {
        crate::test_util::trace_init();
        const FIXTURE_ROOT: &str = "tests/data/fixtures/log_event";

        trace!(?FIXTURE_ROOT, "Opening.");
        std::fs::read_dir(FIXTURE_ROOT)
            .unwrap()
            .for_each(|fixture_file| match fixture_file {
                Ok(fixture_file) => {
                    let path = fixture_file.path();
                    tracing::trace!(?path, "Opening.");
                    let serde_value = open_fixture(&path).unwrap();

                    let vector_value = LogEvent::try_from(serde_value.clone()).unwrap();
                    let serde_value_again: serde_json::Value =
                        vector_value.clone().try_into().unwrap();

                    tracing::trace!(
                        ?path,
                        ?serde_value,
                        ?vector_value,
                        ?serde_value_again,
                        "Asserting equal."
                    );
                    assert_eq!(serde_value, serde_value_again);
                }
                _ => panic!("This test should never read Err'ing test fixtures."),
            });
    }

    // We use `serde_json` pointers in this test to ensure we're validating that Vector correctly inputs and outputs things as expected.
    #[test]
    fn entry() {
        crate::test_util::trace_init();
        let fixture =
            open_fixture("tests/data/fixtures/log_event/motivatingly-complex.json").unwrap();
        let mut event = LogEvent::try_from(fixture).unwrap();

        let lookup = LookupBuf::from_str("non-existing").unwrap();
        let entry = event.entry(lookup).unwrap();
        let fallback = json!(
            "If you don't see this, the `LogEvent::entry` API is not working on non-existing lookups."
        );
        entry.or_insert_with(|| fallback.clone().into());
        let json: serde_json::Value = event.clone().try_into().unwrap();
        trace!(?json);
        assert_eq!(json.pointer("/non-existing"), Some(&fallback));

        let lookup = LookupBuf::from_str("nulled").unwrap();
        let entry = event.entry(lookup).unwrap();
        let fallback = json!(
            "If you see this, the `LogEvent::entry` API is not working on existing, single segment lookups."
        );
        entry.or_insert_with(|| fallback.clone().into());
        let json: serde_json::Value = event.clone().try_into().unwrap();
        assert_eq!(json.pointer("/nulled"), Some(&serde_json::Value::Null));

        let lookup = LookupBuf::from_str("map.basic").unwrap();
        let entry = event.entry(lookup).unwrap();
        let fallback = json!(
            "If you see this, the `LogEvent::entry` API is not working on existing, double segment lookups."
        );
        entry.or_insert_with(|| fallback.clone().into());
        let json: serde_json::Value = event.clone().try_into().unwrap();
        assert_eq!(
            json.pointer("/map/basic"),
            Some(&serde_json::Value::Bool(true))
        );

        let lookup = LookupBuf::from_str("map.map.buddy").unwrap();
        let entry = event.entry(lookup).unwrap();
        let fallback = json!(
            "If you see this, the `LogEvent::entry` API is not working on existing, multi-segment lookups."
        );
        entry.or_insert_with(|| fallback.clone().into());
        let json: serde_json::Value = event.clone().try_into().unwrap();
        assert_eq!(
            json.pointer("/map/map/buddy"),
            Some(&serde_json::Value::Number((-1).into()))
        );

        let lookup = LookupBuf::from_str("map.map.non-existing").unwrap();
        let entry = event.entry(lookup).unwrap();
        let fallback = json!(
            "If you don't see this, the `LogEvent::entry` API is not working on non-existing multi-segment lookups."
        );
        entry.or_insert_with(|| fallback.clone().into());
        let json: serde_json::Value = event.clone().try_into().unwrap();
        assert_eq!(json.pointer("/map/map/non-existing"), Some(&fallback));
    }
}
