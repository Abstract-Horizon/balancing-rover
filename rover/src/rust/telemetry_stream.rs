//
// Copyright (C) 2016-2020 Abstract Horizon
// All rights reserved. This program and the accompanying materials
// are made available under the terms of the Apache License v2.0
// which accompanies this distribution, and is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
//  Contributors:
//    Daniel Sendula - initial API and implementation
//

use std::boxed::Box;
use std::slice::Iter;
use std::io::Write;
use byteorder::{WriteBytesExt, LittleEndian};


pub trait FieldType {
    fn size(&self) -> usize;

    fn type_shortcode(&self) -> &'static str;

    fn definition_to_json(&self, size: usize) -> String;

    fn common_definition_to_json(&self) -> String {
        format!("\"type\" : \"{}\"", self.type_shortcode())
    }
}

pub struct FieldTypeUnsignedByte;
impl FieldType for FieldTypeUnsignedByte {
    fn size(&self) -> usize { 1 }

    fn definition_to_json(&self, _: usize) -> String {
        format!("{}, \"signed\" : \"{}\"", self.common_definition_to_json(), "false")
    }

    fn type_shortcode(&self) -> &'static str { "b" }
}

struct FieldTypeSignedByte;
impl FieldType for FieldTypeSignedByte {
    fn size(&self) -> usize { 1 }

    fn definition_to_json(&self, _: usize) -> String {
        format!("{}, \"signed\" : \"{}\"", self.common_definition_to_json(), "true")
    }

    fn type_shortcode(&self) -> &'static str { "b" }
}


struct FieldTypeUnsignedWord;
impl FieldType for FieldTypeUnsignedWord {
    fn size(&self) -> usize { 2 }

    fn type_shortcode(&self) -> &'static str { "w" }

    fn definition_to_json(&self, _: usize) -> String {
        format!("{}, \"signed\" : \"{}\"", self.common_definition_to_json(), "false")
    }
}


struct FieldTypeSignedWord;
impl FieldType for FieldTypeSignedWord {
    fn size(&self) -> usize { 2 }

    fn type_shortcode(&self) -> &'static str { "w" }

    fn definition_to_json(&self, _: usize) -> String {
        format!("{}, \"signed\" : \"{}\"", self.common_definition_to_json(), "true")
    }
}


struct FieldTypeUnsignedInteger;
impl FieldType for FieldTypeUnsignedInteger {
    fn size(&self) -> usize { 4 }

    fn type_shortcode(&self) -> &'static str { "i" }

    fn definition_to_json(&self, _: usize) -> String {
        format!("{}, \"signed\" : \"{}\"", self.common_definition_to_json(), "false")
    }
}


struct FieldTypeSignedInteger;
impl FieldType for FieldTypeSignedInteger {
    fn size(&self) -> usize { 4 }

    fn type_shortcode(&self) -> &'static str { "i" }

    fn definition_to_json(&self, _: usize) -> String {
        format!("{}, \"signed\" : \"{}\"", self.common_definition_to_json(), "true")
    }
}


struct FieldTypeUnsignedLong;
impl FieldType for FieldTypeUnsignedLong {
    fn size(&self) -> usize { 8 }

    fn type_shortcode(&self) -> &'static str { "l" }

    fn definition_to_json(&self, _: usize) -> String {
        format!("{}, \"signed\" : \"{}\"", self.common_definition_to_json(), "false")
    }
}


struct FieldTypeSignedLong;
impl FieldType for FieldTypeSignedLong {
    fn size(&self) -> usize { 8 }

    fn type_shortcode(&self) -> &'static str { "l" }

    fn definition_to_json(&self, _: usize) -> String {
        format!("{}, \"signed\" : \"{}\"", self.common_definition_to_json(), "true")
    }
}


struct FieldTypeFloat;
impl FieldType for FieldTypeFloat {
    fn size(&self) -> usize { 4 }

    fn type_shortcode(&self) -> &'static str { "f" }

    fn definition_to_json(&self, _: usize) -> String { self.common_definition_to_json() }
}


struct FieldTypeDouble;
impl FieldType for FieldTypeDouble {
    fn size(&self) -> usize { 8 }


    fn type_shortcode(&self) -> &'static str { "d" }

    fn definition_to_json(&self, _: usize) -> String { self.common_definition_to_json() }
}


struct FieldTypeString;
impl FieldType for FieldTypeString {
    fn size(&self) -> usize { 0 }

    fn type_shortcode(&self) -> &'static str { "s" }

    fn definition_to_json(&self, size: usize) -> String {
        format!("{}, \"size\" : \"{}\"", self.common_definition_to_json(), size)
    }
}


struct FieldTypeBytes;
impl FieldType for FieldTypeBytes {
    fn size(&self) -> usize { 0 }

    fn type_shortcode(&self) -> &'static str { "a" }

    fn definition_to_json(&self, size: usize) -> String {
        format!("{}, \"size\" : \"{}\"", self.common_definition_to_json(), size)
    }
}



pub trait Storable {
    fn store(&self, buf: &mut Vec<u8>);
}

impl Storable for u8 {
    fn store(&self, buf: &mut Vec<u8>) { buf.push(*self); }
}

impl Storable for i8 {
    fn store(&self, buf: &mut Vec<u8>) { buf.push(*self as u8); }
}

impl Storable for u16 {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write_u16::<LittleEndian>(*self); }
}

impl Storable for i16 {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write_i16::<LittleEndian>(*self); }
}

impl Storable for u32 {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write_u32::<LittleEndian>(*self); }
}

impl Storable for i32 {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write_i32::<LittleEndian>(*self); }
}

impl Storable for u64 {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write_u64::<LittleEndian>(*self); }
}

impl Storable for i64 {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write_i64::<LittleEndian>(*self); }
}

impl Storable for f32 {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write_f32::<LittleEndian>(*self); }
}

impl Storable for f64 {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write_f64::<LittleEndian>(*self); }
}

impl Storable for &String {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write(self.as_bytes()); }
}

impl Storable for &Vec<u8> {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write(self); }
}

impl Storable for &[u8] {
    fn store(&self, buf: &mut Vec<u8>) { let _ = buf.write(self); }
}


// ----------------------------------------------------------------------------------------------------------


pub struct TelemetryStreamFieldStruct<T: FieldType> {
    field_name: &'static str,
    field_size: usize,
    pub field_type: T
}


pub trait TelemetryStreamField {
    fn name(&self) -> &'static str;
    fn to_json(&self) -> String;
    fn size(&self) -> usize;
}


impl<T: FieldType> TelemetryStreamField for TelemetryStreamFieldStruct<T> {
    fn name(&self) -> &'static str { self.field_name }
    fn to_json(&self) -> String {
       self.field_type.definition_to_json(self.field_size)
    }
    fn size(&self) -> usize { self.field_size }
}


// ----------------------------------------------------------------------------------------------------------

pub struct TelemetryStreamDefinition {
    pub name: &'static str,
    stream_id: u32,
    fixed_length: usize,
    header: Vec<u8>,
    fields:Vec<Box<dyn TelemetryStreamField + Sync + Send>>
}

impl TelemetryStreamDefinition {
    pub fn new(name: &'static str, stream_id: u32, fields: Vec<Box<dyn TelemetryStreamField + Sync + Send>>) -> TelemetryStreamDefinition {
        let fixed_length: usize = fields.iter().map(|field| field.size()).sum();
        let fixed_length = fixed_length + 8; // extra time field at the beginning of record
        let mut header : Vec<u8> = Vec::new();

        let header_byte = if stream_id < 256 { 0 } else { 1 } + if fixed_length < 256 { 0 } else if fixed_length < 65536 { 2 } else { 4 };

        header.push(header_byte);

        if stream_id < 256 {
            header.push(stream_id as u8);
        } else {
            let _ = header.write_u16::<LittleEndian>(stream_id as u16);
        }
        if fixed_length < 256 {
            header.push(fixed_length as u8);
        } else if fixed_length < 655356 {
            let _ = header.write_u16::<LittleEndian>(fixed_length as u16);
        } else {
            let _ = header.write_u32::<LittleEndian>(fixed_length as u32);
        }

        TelemetryStreamDefinition {
            name,
            stream_id,
            fields,
            fixed_length,
            header
        }
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn to_json(&self) -> String {
        let mut s = String::from("");
        let mut first = true;
        for field in self.fields.iter() {
            if first { first = false; } else { s.push_str(", ") }
            s.push_str(format!("\"{}\" : {{ {} }}", field.name(), field.to_json()).as_str());
        }
        format!("{{ \"id\" : {}, \"name\" : \"{}\", \"fields\" : {{ {} }} }}", self.stream_id, self.name, s)
    }

    pub fn size(&self) -> usize {
        self.fixed_length + self.header.len() // time f64
    }

    pub fn write_header(&self, buf: &mut Vec<u8>) {
        let _ = buf.write(&self.header);
    }

    pub fn fields(&self) -> Iter<Box<dyn TelemetryStreamField + Sync + Send>> {
        self.fields.iter()
    }

    #[allow(dead_code)]
    pub fn unsigned_byte_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeUnsignedByte> {
            field_name: name,
            field_type: FieldTypeUnsignedByte,
            field_size: FieldTypeUnsignedByte.size(),
        })
    }

    #[allow(dead_code)]
    pub fn signed_byte_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeSignedByte> {
            field_name: name,
            field_type: FieldTypeSignedByte,
            field_size: FieldTypeSignedByte.size(),
        })
    }

    #[allow(dead_code)]
    pub fn unsigned_word_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeUnsignedWord> {
            field_name: name,
            field_type: FieldTypeUnsignedWord,
            field_size: FieldTypeUnsignedWord.size(),
        })
    }

    #[allow(dead_code)]
    pub fn signed_word_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeSignedWord> {
            field_name: name,
            field_type: FieldTypeSignedWord,
            field_size: FieldTypeSignedWord.size(),
        })
    }

    #[allow(dead_code)]
    pub fn unsigned_integer_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeUnsignedInteger> {
            field_name: name,
            field_type: FieldTypeUnsignedInteger,
            field_size: FieldTypeUnsignedInteger.size(),
        })
    }

    #[allow(dead_code)]
    pub fn signed_integer_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeSignedInteger> {
            field_name: name,
            field_type: FieldTypeSignedInteger,
            field_size: FieldTypeSignedInteger.size(),
        })
    }

    #[allow(dead_code)]
    pub fn unsigned_long_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeUnsignedLong> {
            field_name: name,
            field_type: FieldTypeUnsignedLong,
            field_size: FieldTypeUnsignedLong.size(),
        })
    }

    #[allow(dead_code)]
    pub fn signed_long_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeSignedLong> {
            field_name: name,
            field_type: FieldTypeSignedLong,
            field_size: FieldTypeSignedLong.size(),
        })
    }

    #[allow(dead_code)]
    pub fn float_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeFloat> {
            field_name: name,
            field_type: FieldTypeFloat,
            field_size: FieldTypeFloat.size(),
        })
    }

    #[allow(dead_code)]
    pub fn double_field(name: &'static str) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeDouble> {
            field_name: name,
            field_type: FieldTypeDouble,
            field_size: FieldTypeDouble.size(),
        })
    }

    #[allow(dead_code)]
    pub fn string_field(name: &'static str, string_size: usize) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeString> {
            field_name: name,
            field_type: FieldTypeString,
            field_size: string_size,
        })
    }

    #[allow(dead_code)]
    pub fn bytes_field(name: &'static str, bytes_size: usize) -> Box<dyn TelemetryStreamField + Sync + Send> {
        Box::new(TelemetryStreamFieldStruct::<FieldTypeBytes> {
            field_name: name,
            field_type: FieldTypeBytes,
            field_size: bytes_size,
        })
    }
}
