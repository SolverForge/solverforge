//! Serde support for `BendableScore`.
//!
//! serde implements its traits for `[T; N]` only for fixed lengths up to 32, so
//! the derive cannot be used on the const-generic level arrays. These impls give
//! the same wire shape the derive gives every other score type,
//! `{"hard": [...], "soft": [...]}`, and fill the arrays in place: a level list
//! whose length is not exactly `H` (or `S`) is rejected with a length error.

use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::BendableScore;

const FIELDS: &[&str] = &["hard", "soft"];

impl<const H: usize, const S: usize> Serialize for BendableScore<H, S> {
    fn serialize<Ser: Serializer>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error> {
        let mut state = serializer.serialize_struct("BendableScore", 2)?;
        state.serialize_field("hard", &self.hard_scores()[..])?;
        state.serialize_field("soft", &self.soft_scores()[..])?;
        state.end()
    }
}

impl<'de, const H: usize, const S: usize> Deserialize<'de> for BendableScore<H, S> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_struct("BendableScore", FIELDS, ScoreVisitor::<H, S>)
    }
}

struct ScoreVisitor<const H: usize, const S: usize>;

impl<'de, const H: usize, const S: usize> Visitor<'de> for ScoreVisitor<H, S> {
    type Value = BendableScore<H, S>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a BendableScore with {H} hard and {S} soft levels"
        )
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let hard = seq
            .next_element_seed(Levels::<H>(PhantomData))?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let soft = seq
            .next_element_seed(Levels::<S>(PhantomData))?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        Ok(BendableScore::of(hard, soft))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut hard = None;
        let mut soft = None;
        while let Some(key) = map.next_key::<Field>()? {
            match key {
                Field::Hard if hard.is_some() => return Err(de::Error::duplicate_field("hard")),
                Field::Hard => hard = Some(map.next_value_seed(Levels::<H>(PhantomData))?),
                Field::Soft if soft.is_some() => return Err(de::Error::duplicate_field("soft")),
                Field::Soft => soft = Some(map.next_value_seed(Levels::<S>(PhantomData))?),
                Field::Other => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        let hard = hard.ok_or_else(|| de::Error::missing_field("hard"))?;
        let soft = soft.ok_or_else(|| de::Error::missing_field("soft"))?;
        Ok(BendableScore::of(hard, soft))
    }
}

enum Field {
    Hard,
    Soft,
    Other,
}

impl<'de> Deserialize<'de> for Field {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldVisitor;

        impl Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("`hard` or `soft`")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Field, E> {
                Ok(match value {
                    "hard" => Field::Hard,
                    "soft" => Field::Soft,
                    _ => Field::Other,
                })
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)
    }
}

/// Reads one level list into `[i64; N]`, requiring exactly `N` elements.
struct Levels<const N: usize>(PhantomData<fn() -> [i64; N]>);

impl<'de, const N: usize> DeserializeSeed<'de> for Levels<N> {
    type Value = [i64; N];

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<[i64; N], D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, const N: usize> Visitor<'de> for Levels<N> {
    type Value = [i64; N];

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a list of exactly {N} score levels")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<[i64; N], A::Error> {
        let mut levels = [0; N];
        for (index, level) in levels.iter_mut().enumerate() {
            *level = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(index, &self))?;
        }
        let mut length = N;
        while seq.next_element::<IgnoredAny>()?.is_some() {
            length += 1;
        }
        if length != N {
            return Err(de::Error::invalid_length(length, &self));
        }
        Ok(levels)
    }
}
