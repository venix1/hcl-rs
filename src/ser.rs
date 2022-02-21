use crate::{Attribute, Block, BlockLabel, Body, Result, Structure, Value};
use std::io;

pub struct Serializer<'a, W> {
    writer: W,
    formatter: Formatter<'a>,
}

impl<'a, W> Serializer<'a, W>
where
    W: io::Write,
{
    pub fn new(writer: W) -> Serializer<'a, W> {
        Serializer::with_formatter(writer, Formatter::default())
    }

    pub fn with_formatter(writer: W, formatter: Formatter<'a>) -> Serializer<'a, W> {
        Serializer { writer, formatter }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    pub fn serialize(&mut self, body: &Body) -> Result<()> {
        self.serialize_body(body)?;
        Ok(())
    }

    fn serialize_body(&mut self, body: &Body) -> io::Result<()> {
        #[derive(PartialEq)]
        enum State {
            First,
            Attribute,
            Block,
        }

        let mut state = State::First;

        for structure in body.iter() {
            match structure {
                Structure::Attribute(attr) => {
                    let collection = attr.value.is_array() || attr.value.is_object();

                    if state == State::Block || (state == State::Attribute && collection) {
                        self.writer.write_all(b"\n")?;
                    }

                    self.serialize_attribute(attr)?;
                    state = State::Attribute;
                }
                Structure::Block(block) => {
                    if state != State::First {
                        self.formatter.write_empty_line(&mut self.writer)?;
                    }

                    self.serialize_block(block)?;
                    state = State::Block;
                }
            }

            self.formatter.end_object_value()?;
        }

        Ok(())
    }

    fn serialize_attribute(&mut self, attr: &Attribute) -> io::Result<()> {
        self.formatter.begin_object_key(&mut self.writer)?;
        self.writer.write_all(attr.key.as_bytes())?;
        self.formatter.begin_object_value(&mut self.writer)?;
        self.serialize_value(&attr.value)
    }

    fn serialize_block(&mut self, block: &Block) -> io::Result<()> {
        self.writer.write_all(block.identifier.as_bytes())?;
        self.writer.write_all(b" ")?;

        for label in block.labels.iter() {
            self.serialize_block_label(label)?;
            self.writer.write_all(b" ")?;
        }

        self.formatter.begin_object(&mut self.writer)?;
        self.serialize_body(&block.body)?;
        self.formatter.end_object(&mut self.writer)
    }

    fn serialize_block_label(&mut self, label: &BlockLabel) -> io::Result<()> {
        match label {
            BlockLabel::Identifier(identifier) => {
                self.writer.write_all(identifier.as_bytes())?;
            }
            BlockLabel::StringLit(s) => {
                self.serialize_str(s)?;
            }
        }

        Ok(())
    }

    fn serialize_value(&mut self, value: &Value) -> io::Result<()> {
        match value {
            Value::Null => {
                self.writer.write_all(b"null")?;
            }
            Value::Bool(b) => {
                self.writer.write_all(if *b { b"true" } else { b"false" })?;
            }
            Value::Number(n) => {
                self.writer.write_all(n.to_string().as_bytes())?;
            }
            Value::String(s) => {
                self.serialize_str(s)?;
            }
            Value::Array(array) => {
                self.formatter.begin_array(&mut self.writer)?;

                for (i, value) in array.iter().enumerate() {
                    self.formatter.begin_array_value(&mut self.writer, i == 0)?;
                    self.serialize_value(value)?;
                    self.formatter.end_array_value()?;
                }

                self.formatter.end_array(&mut self.writer)?;
            }
            Value::Object(object) => {
                self.formatter.begin_object(&mut self.writer)?;

                for (key, value) in object.iter() {
                    self.formatter.begin_object_key(&mut self.writer)?;
                    self.serialize_str(key)?;
                    self.formatter.begin_object_value(&mut self.writer)?;
                    self.serialize_value(value)?;
                    self.formatter.end_object_value()?;
                }

                self.formatter.end_object(&mut self.writer)?;
            }
        }

        Ok(())
    }

    fn serialize_str(&mut self, s: &str) -> io::Result<()> {
        self.formatter.begin_string(&mut self.writer)?;
        self.writer.write_all(s.as_bytes())?;
        self.formatter.end_string(&mut self.writer)
    }
}

pub struct Formatter<'a> {
    current_indent: usize,
    has_value: bool,
    indent: &'a [u8],
}

impl<'a> Default for Formatter<'a> {
    fn default() -> Formatter<'a> {
        Formatter::with_indent(b"  ")
    }
}

impl<'a> Formatter<'a> {
    pub fn with_indent(indent: &'a [u8]) -> Formatter<'a> {
        Formatter {
            current_indent: 0,
            has_value: false,
            indent,
        }
    }

    fn begin_string<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"\"")
    }

    fn end_string<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"\"")
    }

    fn write_empty_line<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"\n\n")?;
        indent(writer, self.current_indent, self.indent)
    }

    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"[")
    }

    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;

        if self.has_value {
            writer.write_all(b"\n")?;
            indent(writer, self.current_indent, self.indent)?;
        }

        writer.write_all(b"]")
    }

    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first {
            writer.write_all(b"\n")?;
        } else {
            writer.write_all(b",\n")?;
        }
        indent(writer, self.current_indent, self.indent)
    }

    fn end_array_value(&mut self) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }

    fn begin_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"{")
    }

    fn end_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;

        if self.has_value {
            writer.write_all(b"\n")?;
            indent(writer, self.current_indent, self.indent)?;
        }

        writer.write_all(b"}")
    }

    fn begin_object_key<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"\n")?;
        indent(writer, self.current_indent, self.indent)
    }

    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b" = ")
    }

    fn end_object_value(&mut self) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }
}

pub fn to_vec(body: &Body) -> Result<Vec<u8>> {
    let mut vec = Vec::with_capacity(128);
    to_writer(&mut vec, body)?;
    Ok(vec)
}

pub fn to_string(body: &Body) -> Result<String> {
    let vec = to_vec(body)?;
    let string = unsafe {
        // We do not emit invalid UTF-8.
        String::from_utf8_unchecked(vec)
    };
    Ok(string)
}

pub fn to_writer<W>(writer: W, body: &Body) -> Result<()>
where
    W: io::Write,
{
    let mut serializer = Serializer::new(writer);
    serializer.serialize(body)
}

fn indent<W>(writer: &mut W, n: usize, s: &[u8]) -> io::Result<()>
where
    W: ?Sized + io::Write,
{
    for _ in 0..n {
        writer.write_all(s)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Map;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_to_string() {
        let mut tags = Map::new();
        tags.insert("Environment".into(), "production".into());
        tags.insert("Num".into(), 1.5f64.into());

        let body = Body::builder()
            .add_block(
                Block::builder("resource")
                    .add_label("aws_s3_bucket")
                    .add_label("bucket")
                    .add_attribute(Attribute::new("name", "the-bucket"))
                    .add_attribute(Attribute::new("force_destroy", true))
                    .add_attribute(Attribute::new("tags", tags))
                    .add_block(
                        Block::builder("logging")
                            .add_attribute(Attribute::new("target_bucket", "the-target"))
                            .build(),
                    )
                    .build(),
            )
            .build();

        let expected = r#"resource "aws_s3_bucket" "bucket" {
  name = "the-bucket"
  force_destroy = true

  tags = {
    "Environment" = "production"
    "Num" = 1.5
  }

  logging {
    target_bucket = "the-target"
  }
}"#;

        assert_eq!(to_string(&body).unwrap(), expected);
    }
}
