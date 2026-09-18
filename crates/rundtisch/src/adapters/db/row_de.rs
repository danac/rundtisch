//! Shared serde decoder for database rows.
//!
//! Adapters implement [`RowCells`] with backend-specific reads. Conversion to
//! the model field type is type-directed so SQLite and D1 agree for a given
//! `#[derive(Serialize, Deserialize)]` struct.

use serde::de::{
    self, DeserializeOwned, DeserializeSeed, Deserializer, IntoDeserializer, MapAccess, SeqAccess,
    Visitor,
};
use serde::forward_to_deserialize_any;

use crate::traits::db::{Error, Result};

pub(crate) trait RowCells {
    fn is_null(&self, col: &str) -> Result<bool>;
    fn get_bool(&self, col: &str) -> Result<bool>;
    fn get_i64(&self, col: &str) -> Result<i64>;
    fn get_f64(&self, col: &str) -> Result<f64>;
    fn get_string(&self, col: &str) -> Result<String>;
    fn get_bytes(&self, col: &str) -> Result<Vec<u8>>;
}

pub(crate) fn from_row<T: DeserializeOwned>(row: &impl RowCells) -> Result<T> {
    T::deserialize(RowDeserializer { row })
}

pub(crate) fn integer_valued_f64(n: f64) -> Result<i64> {
    if !n.is_finite() || n.fract() != 0.0 {
        return Err(Error::TypeMismatch);
    }
    if n < i64::MIN as f64 || n > i64::MAX as f64 {
        return Err(Error::TypeMismatch);
    }
    let i = n as i64;
    if i as f64 == n {
        Ok(i)
    } else {
        Err(Error::TypeMismatch)
    }
}

pub(crate) fn i64_as_bool(v: i64) -> Result<bool> {
    match v {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::TypeMismatch),
    }
}

struct RowDeserializer<'a, R: RowCells> {
    row: &'a R,
}

impl<'de, R: RowCells> Deserializer<'de> for RowDeserializer<'_, R> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        Err(de::Error::invalid_type(de::Unexpected::Map, &visitor))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_map(RowMap {
            row: self.row,
            fields,
            index: 0,
        })
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }
}

struct RowMap<'a, R: RowCells> {
    row: &'a R,
    fields: &'static [&'static str],
    index: usize,
}

impl<'de, R: RowCells> MapAccess<'de> for RowMap<'_, R> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.index >= self.fields.len() {
            return Ok(None);
        }
        let field = self.fields[self.index];
        self.index += 1;
        seed.deserialize(field.into_deserializer()).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let col = self.fields[self.index - 1];
        seed.deserialize(CellDeserializer { row: self.row, col })
    }
}

struct CellDeserializer<'a, R: RowCells> {
    row: &'a R,
    col: &'a str,
}

impl<R: RowCells> CellDeserializer<'_, R> {
    fn i64(&self) -> Result<i64> {
        self.row.get_i64(self.col)
    }

    fn f64(&self) -> Result<f64> {
        self.row.get_f64(self.col)
    }
}

impl<'de, R: RowCells> Deserializer<'de> for CellDeserializer<'_, R> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.row.is_null(self.col)? {
            return visitor.visit_none();
        }
        if let Ok(v) = self.row.get_bool(self.col) {
            return visitor.visit_bool(v);
        }
        if let Ok(v) = self.row.get_i64(self.col) {
            return visitor.visit_i64(v);
        }
        if let Ok(v) = self.row.get_f64(self.col) {
            return visitor.visit_f64(v);
        }
        if let Ok(v) = self.row.get_string(self.col) {
            return visitor.visit_string(v);
        }
        if let Ok(v) = self.row.get_bytes(self.col) {
            return visitor.visit_byte_buf(v);
        }
        Err(Error::TypeMismatch)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_bool(self.row.get_bool(self.col)?)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i8(i8::try_from(self.i64()?).map_err(|_| Error::TypeMismatch)?)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i16(i16::try_from(self.i64()?).map_err(|_| Error::TypeMismatch)?)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i32(i32::try_from(self.i64()?).map_err(|_| Error::TypeMismatch)?)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i64(self.i64()?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u8(u8::try_from(self.i64()?).map_err(|_| Error::TypeMismatch)?)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u16(u16::try_from(self.i64()?).map_err(|_| Error::TypeMismatch)?)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u32(u32::try_from(self.i64()?).map_err(|_| Error::TypeMismatch)?)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u64(u64::try_from(self.i64()?).map_err(|_| Error::TypeMismatch)?)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_f32(self.f64()? as f32)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_f64(self.f64()?)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.row.get_string(self.col)?;
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(Error::TypeMismatch),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_string(self.row.get_string(self.col)?)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_byte_buf(self.row.get_bytes(self.col)?)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.row.is_null(self.col)? {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.row.is_null(self.col)? {
            visitor.visit_unit()
        } else {
            Err(Error::TypeMismatch)
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(BytesSeq(self.row.get_bytes(self.col)?.into_iter()))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_enum(self.row.get_string(self.col)?.into_deserializer())
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        i128 u128 unit_struct newtype_struct tuple tuple_struct map struct
    }
}

struct BytesSeq(std::vec::IntoIter<u8>);

impl<'de> SeqAccess<'de> for BytesSeq {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        match self.0.next() {
            Some(b) => seed.deserialize(b.into_deserializer()).map(Some),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Clone)]
    enum Cell {
        Null,
        Bool(bool),
        Int(i64),
        Float(f64),
        Text(String),
        Bytes(Vec<u8>),
    }

    struct MapRow(HashMap<&'static str, Cell>);

    impl RowCells for MapRow {
        fn is_null(&self, col: &str) -> Result<bool> {
            match self.0.get(col) {
                Some(Cell::Null) => Ok(true),
                Some(_) => Ok(false),
                None => Err(Error::Backend(format!("missing column: {col}"))),
            }
        }

        fn get_bool(&self, col: &str) -> Result<bool> {
            match self.0.get(col) {
                Some(Cell::Bool(v)) => Ok(*v),
                Some(Cell::Int(v)) => i64_as_bool(*v),
                Some(Cell::Float(v)) => i64_as_bool(integer_valued_f64(*v)?),
                _ => Err(Error::TypeMismatch),
            }
        }

        fn get_i64(&self, col: &str) -> Result<i64> {
            match self.0.get(col) {
                Some(Cell::Int(v)) => Ok(*v),
                Some(Cell::Bool(v)) => Ok(i64::from(*v)),
                Some(Cell::Float(v)) => integer_valued_f64(*v),
                _ => Err(Error::TypeMismatch),
            }
        }

        fn get_f64(&self, col: &str) -> Result<f64> {
            match self.0.get(col) {
                Some(Cell::Float(v)) => Ok(*v),
                Some(Cell::Int(v)) => Ok(*v as f64),
                _ => Err(Error::TypeMismatch),
            }
        }

        fn get_string(&self, col: &str) -> Result<String> {
            match self.0.get(col) {
                Some(Cell::Text(v)) => Ok(v.clone()),
                _ => Err(Error::TypeMismatch),
            }
        }

        fn get_bytes(&self, col: &str) -> Result<Vec<u8>> {
            match self.0.get(col) {
                Some(Cell::Bytes(v)) => Ok(v.clone()),
                _ => Err(Error::TypeMismatch),
            }
        }
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct Item {
        id: i64,
        score: f64,
        flag: bool,
        name: Option<String>,
        blob: Vec<u8>,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    enum Kind {
        Admin,
        User,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct Tagged {
        kind: Kind,
    }

    #[test]
    fn integer_valued_f64_accepts_whole_numbers_only() {
        assert_eq!(integer_valued_f64(0.0).unwrap(), 0);
        assert_eq!(integer_valued_f64(-3.0).unwrap(), -3);
        assert!(integer_valued_f64(1.5).is_err());
        assert!(integer_valued_f64(f64::NAN).is_err());
        assert!(integer_valued_f64(f64::INFINITY).is_err());
    }

    #[test]
    fn i64_as_bool_only_zero_and_one() {
        assert_eq!(i64_as_bool(0).unwrap(), false);
        assert_eq!(i64_as_bool(1).unwrap(), true);
        assert!(i64_as_bool(2).is_err());
        assert!(i64_as_bool(-1).is_err());
    }

    #[test]
    fn from_row_coerces_int_float_null_and_bytes() {
        let row = MapRow(HashMap::from([
            ("id", Cell::Int(7)),
            ("score", Cell::Int(4)),
            ("flag", Cell::Int(1)),
            ("name", Cell::Null),
            ("blob", Cell::Bytes(vec![1, 2])),
        ]));
        let item: Item = from_row(&row).unwrap();
        assert_eq!(
            item,
            Item {
                id: 7,
                score: 4.0,
                flag: true,
                name: None,
                blob: vec![1, 2],
            }
        );
    }

    #[test]
    fn from_row_coerces_whole_float_to_i64_and_bool() {
        let row = MapRow(HashMap::from([
            ("id", Cell::Float(9.0)),
            ("score", Cell::Float(1.25)),
            ("flag", Cell::Float(0.0)),
            ("name", Cell::Text("x".into())),
            ("blob", Cell::Bytes(Vec::new())),
        ]));
        let item: Item = from_row(&row).unwrap();
        assert_eq!(item.id, 9);
        assert_eq!(item.score, 1.25);
        assert_eq!(item.flag, false);
        assert_eq!(item.name.as_deref(), Some("x"));
    }

    #[test]
    fn from_row_rejects_fractional_float_as_i64() {
        let row = MapRow(HashMap::from([
            ("id", Cell::Float(1.5)),
            ("score", Cell::Float(0.0)),
            ("flag", Cell::Bool(true)),
            ("name", Cell::Null),
            ("blob", Cell::Bytes(Vec::new())),
        ]));
        assert_eq!(from_row::<Item>(&row).unwrap_err(), Error::TypeMismatch);
    }

    #[test]
    fn from_row_unit_enum_from_text() {
        let row = MapRow(HashMap::from([("kind", Cell::Text("Admin".into()))]));
        let tagged: Tagged = from_row(&row).unwrap();
        assert_eq!(tagged.kind, Kind::Admin);
    }
}
