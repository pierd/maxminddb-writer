use serde::ser;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Custom(String),
    UnknownLength,
    LengthOutOfRange,
    IntegerOutOfRange,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Custom(msg.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::IO(ref err) => write!(f, "IO error: {}", err),
            Error::Custom(ref err) => write!(f, "Custom error: {}", err),
            Error::UnknownLength => write!(f, "Unknown length"),
            Error::LengthOutOfRange => write!(f, "Length out of range"),
            Error::IntegerOutOfRange => write!(f, "Integer out of range"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum TypeId {
    // Pointer = 1,
    String = 2,
    Double = 3,
    Bytes = 4,
    Uint16 = 5,
    Uint32 = 6,
    Map = 7,
    Int32 = 8,
    Uint64 = 9,
    Uint128 = 10,
    Array = 11,
    // Container = 12,
    // EndMarker = 13,
    Boolean = 14,
    Float = 15,
}

pub struct Serializer<W> {
    writer: W,
}

impl<W> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Serializer { writer }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    fn write_control(&mut self, type_id: TypeId, size: usize) -> Result<(), Error>
    where
        W: std::io::Write,
    {
        // check if the size will fit
        if size > 16_843_036 {
            return Err(Error::LengthOutOfRange);
        }

        // split the type into possibly 2 bytes
        let (first, second) = if type_id as usize <= 7 {
            ((type_id as u8) << 5, None)
        } else {
            (0, Some((type_id as u8) - 7))
        };

        // write the first byte and calculate the leftover size
        let (bytes_count, leftover_size) = if size < 29 {
            self.writer.write_all(&[first | (size as u8)])?;
            (0, 0)
        } else if size < 285 {
            self.writer.write_all(&[first | 29])?;
            (1, size - 29)
        } else if size < 65821 {
            self.writer.write_all(&[first | 30])?;
            (2, size - 285)
        } else {
            self.writer.write_all(&[first | 31])?;
            (3, size - 65821)
        };

        // write the second byte if needed
        if let Some(second) = second {
            self.writer.write_all(&[second])?;
        }

        // write the leftover size
        let bytes = leftover_size.to_be_bytes();
        self.writer
            .write_all(&bytes[(bytes.len() - bytes_count)..bytes.len()])?;

        Ok(())
    }

    fn serialize<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: ser::Serialize,
        W: std::io::Write,
    {
        value.serialize(self)
    }
}

impl<W> ser::Serializer for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = Self;

    type SerializeStruct = Self;

    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.write_control(TypeId::Boolean, if v { 1 } else { 0 })?;
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        // FIXME
        self.write_control(TypeId::Int32, 4)?;
        self.writer.write_all(&v.to_be_bytes())?;
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        let v: i32 = v.try_into().map_err(|_| Error::IntegerOutOfRange)?;
        self.serialize_i32(v)
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        let v: i32 = v.try_into().map_err(|_| Error::IntegerOutOfRange)?;
        self.serialize_i32(v)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u16(v as u16)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        v.as_big_endian_slice(|buf| {
            self.write_control(TypeId::Uint16, buf.len())?;
            self.writer.write_all(buf)?;
            Ok(())
        })
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        v.as_big_endian_slice(|buf| {
            self.write_control(TypeId::Uint32, buf.len())?;
            self.writer.write_all(buf)?;
            Ok(())
        })
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        v.as_big_endian_slice(|buf| {
            self.write_control(TypeId::Uint64, buf.len())?;
            self.writer.write_all(buf)?;
            Ok(())
        })
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        v.as_big_endian_slice(|buf| {
            self.write_control(TypeId::Uint128, buf.len())?;
            self.writer.write_all(buf)?;
            Ok(())
        })
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.write_control(TypeId::Float, 4)?;
        self.writer.write_all(&v.to_be_bytes())?;
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.write_control(TypeId::Double, 8)?;
        self.writer.write_all(&v.to_be_bytes())?;
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        let buf = v.encode_utf8(&mut buf);
        self.write_control(TypeId::String, buf.len())?;
        self.writer.write_all(buf.as_bytes())?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.write_control(TypeId::String, v.len())?;
        self.writer.write_all(v.as_bytes())?;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.write_control(TypeId::Bytes, v.len())?;
        self.writer.write_all(v)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_bool(false)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_bool(true)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(name)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let Some(len) = len else {
            return Err(Error::UnknownLength);
        };
        self.write_control(TypeId::Array, len)?;
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let Some(len) = len else {
            return Err(Error::UnknownLength);
        };
        self.write_control(TypeId::Map, len)?;
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.serialize_map(Some(len))
    }
}

impl<W> ser::SerializeSeq for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = <Self as ser::Serializer>::Ok;

    type Error = <Self as ser::Serializer>::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W> ser::SerializeTuple for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = <Self as ser::Serializer>::Ok;

    type Error = <Self as ser::Serializer>::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W> ser::SerializeTupleStruct for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = <Self as ser::Serializer>::Ok;

    type Error = <Self as ser::Serializer>::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W> ser::SerializeTupleVariant for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = <Self as ser::Serializer>::Ok;

    type Error = <Self as ser::Serializer>::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

// TODO: do we have to care about the order of calls?
impl<W> ser::SerializeMap for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = <Self as ser::Serializer>::Ok;

    type Error = <Self as ser::Serializer>::Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize(key)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W> ser::SerializeStruct for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = <Self as ser::Serializer>::Ok;

    type Error = <Self as ser::Serializer>::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize(key)?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W> ser::SerializeStructVariant for &mut Serializer<W>
where
    W: std::io::Write,
{
    type Ok = <Self as ser::Serializer>::Ok;

    type Error = <Self as ser::Serializer>::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize(key)?;
        self.serialize(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

trait AsBigEndianSlice {
    fn as_big_endian_slice<R, F: FnMut(&[u8]) -> R>(&self, f: F) -> R;
}

macro_rules! impl_as_big_endian_slice_for {
    ($uint:path) => {
        impl AsBigEndianSlice for $uint {
            fn as_big_endian_slice<R, F: FnMut(&[u8]) -> R>(&self, mut f: F) -> R {
                let bytes = self.to_be_bytes();
                let mut slice = bytes.as_ref();
                while slice.strip_prefix(&[0]).is_some() {
                    slice = &slice[1..];
                }
                f(slice)
            }
        }
    };
}

impl_as_big_endian_slice_for!(u16);
impl_as_big_endian_slice_for!(u32);
impl_as_big_endian_slice_for!(u64);
impl_as_big_endian_slice_for!(u128);

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::Database;

    use super::*;

    fn control(type_id: TypeId, len: usize) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut serializer = Serializer::new(&mut buf);
        serializer.write_control(type_id, len).unwrap();
        buf
    }

    #[test]
    fn test_write_control() {
        assert_eq!(control(TypeId::String, 2), vec![0b01000010]);
        assert_eq!(control(TypeId::String, 28), vec![0b01011100]);
        assert_eq!(control(TypeId::String, 80), vec![0b01011101, 0b00110011]);
        assert_eq!(
            control(TypeId::String, 13392),
            vec![0b01011110, 0b00110011, 0b00110011]
        );
        assert_eq!(
            control(TypeId::String, 3421264),
            vec![0b01011111, 0b00110011, 0b00110011, 0b00110011]
        );
        assert_eq!(
            control(TypeId::String, 16843036),
            vec![0b01011111, 0xFF, 0xFF, 0xFF]
        );

        assert_eq!(control(TypeId::Uint32, 1), vec![0b11000001]);
        assert_eq!(control(TypeId::Uint128, 3), vec![0b00000011, 0b00000011]);
    }

    fn create_minimal_db<T>(value: &T) -> Vec<u8>
    where
        T: serde::Serialize,
    {
        let mut db = Database::default();
        let data = db.insert_value(value).unwrap();
        db.insert_node([false].into_iter(), data);
        db.insert_node([true].into_iter(), data);
        db.to_vec().unwrap()
    }

    fn test_pass_through_maxminddb<T>(value: T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let db = create_minimal_db(&value);
        let reader = maxminddb::Reader::from_source(db.as_slice()).unwrap();
        let deserialized_value: T = reader.lookup([0, 0, 0, 0].try_into().unwrap()).unwrap();
        assert_eq!(value, deserialized_value);
    }

    #[test]
    fn test() {
        test_pass_through_maxminddb(false);
        test_pass_through_maxminddb(true);

        test_pass_through_maxminddb(0u16);
        test_pass_through_maxminddb(0u32);
        test_pass_through_maxminddb(0u64);

        test_pass_through_maxminddb(42u16);
        test_pass_through_maxminddb(42u32);
        test_pass_through_maxminddb(42u64);

        test_pass_through_maxminddb(u16::MAX);
        test_pass_through_maxminddb(u32::MAX);
        test_pass_through_maxminddb(u64::MAX);
        test_pass_through_maxminddb(u128::MAX);

        test_pass_through_maxminddb(0i32);
        test_pass_through_maxminddb(-1i32);
        test_pass_through_maxminddb(i32::MAX);
        test_pass_through_maxminddb(i32::MIN);

        test_pass_through_maxminddb(-42i64);

        test_pass_through_maxminddb("".to_string());
        test_pass_through_maxminddb("test".to_string());
        test_pass_through_maxminddb("zażółć gęślą jaźń".to_string());

        test_pass_through_maxminddb([1, 2, 3]);
        test_pass_through_maxminddb(vec![1, 2, 3]);

        let mut map = HashMap::new();
        map.insert("test".to_string(), 42);
        map.insert("test2".to_string(), 42);
        test_pass_through_maxminddb(map);

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Test {
            a: u32,
            b: String,
            c: Vec<u64>,
        }
        test_pass_through_maxminddb(Test {
            a: 42,
            b: "test".to_string(),
            c: vec![1, 2, 3],
        });
    }
}
