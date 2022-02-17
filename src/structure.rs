use crate::{Map, Value};

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Body(Vec<Structure>);

impl Body {
    pub fn new() -> Body {
        Body::default()
    }

    pub fn into_inner(self) -> Vec<Structure> {
        self.0
    }

    pub fn builder() -> BodyBuilder {
        BodyBuilder::default()
    }

    fn into_map(self) -> Map<String, Node> {
        self.0.into_iter().fold(Map::new(), |mut map, structure| {
            match structure {
                Structure::Attribute(attr) => {
                    map.insert(attr.name, Node::Value(attr.value));
                }
                Structure::Block(block) => {
                    block.into_map().into_iter().for_each(|(key, mut node)| {
                        map.entry(key)
                            .and_modify(|entry| entry.deep_merge(&mut node))
                            .or_insert(node);
                    });
                }
            };

            map
        })
    }
}

impl From<Body> for Value {
    fn from(body: Body) -> Value {
        Value::from_iter(body.into_map())
    }
}

impl FromIterator<Structure> for Body {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Structure>,
    {
        Body(iter.into_iter().collect())
    }
}

impl IntoIterator for Body {
    type Item = Structure;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Default)]
pub struct BodyBuilder(Vec<Structure>);

impl BodyBuilder {
    pub fn add_attribute(mut self, attr: Attribute) -> BodyBuilder {
        self.0.push(Structure::Attribute(attr));
        self
    }

    pub fn add_block(mut self, block: Block) -> BodyBuilder {
        self.0.push(Structure::Block(block));
        self
    }

    pub fn build(self) -> Body {
        Body::from_iter(self.0)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Structure {
    Attribute(Attribute),
    Block(Block),
}

impl From<Structure> for Value {
    fn from(s: Structure) -> Value {
        match s {
            Structure::Attribute(attr) => attr.into(),
            Structure::Block(block) => block.into(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Attribute {
    pub name: String,
    pub value: Value,
}

impl Attribute {
    pub fn new<V>(name: &str, value: V) -> Attribute
    where
        V: Into<Value>,
    {
        Attribute {
            name: name.to_string(),
            value: value.into(),
        }
    }
}

impl From<Attribute> for Value {
    fn from(attr: Attribute) -> Value {
        Value::from_iter(std::iter::once((attr.name, attr.value)))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub name: String,
    pub labels: Vec<String>,
    pub body: Body,
}

impl Block {
    pub fn new<I>(name: &str, labels: I, body: Body) -> Block
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Block {
            name: name.to_string(),
            labels: labels.into_iter().map(Into::into).collect(),
            body: body,
        }
    }

    pub fn builder(name: &str) -> BlockBuilder {
        BlockBuilder::new(name)
    }

    fn into_map(self) -> Map<String, Node> {
        let mut labels = self.labels.into_iter();

        let node = match labels.next() {
            Some(name) => {
                let block = Block {
                    name,
                    labels: labels.collect(),
                    body: self.body,
                };

                Node::Block(block.into_map())
            }
            None => Node::BlockBodies(vec![self.body]),
        };

        Map::from_iter(std::iter::once((self.name, node)))
    }
}

impl From<Block> for Value {
    fn from(block: Block) -> Value {
        Value::from_iter(block.into_map())
    }
}

enum Node {
    Empty,
    Block(Map<String, Node>),
    BlockBodies(Vec<Body>),
    Value(Value),
}

impl From<Node> for Value {
    fn from(node: Node) -> Value {
        match node {
            Node::Empty => Value::Null,
            Node::Block(map) => Value::from_iter(map),
            Node::BlockBodies(mut vec) => {
                // Flatten as per the [HCL JSON spec](json-spec)
                //
                // [json-spec]: https://github.com/hashicorp/hcl/blob/main/json/spec.md#blocks
                if vec.len() == 1 {
                    vec.remove(0).into()
                } else {
                    vec.into()
                }
            }
            Node::Value(value) => value,
        }
    }
}

impl Node {
    fn take(&mut self) -> Node {
        std::mem::replace(self, Node::Empty)
    }

    fn deep_merge(&mut self, other: &mut Node) {
        match (self, other) {
            (Node::Block(lhs), Node::Block(rhs)) => {
                rhs.iter_mut().for_each(|(key, node)| {
                    lhs.entry(key.to_string())
                        .and_modify(|lhs| lhs.deep_merge(node))
                        .or_insert_with(|| node.take());
                });
            }
            (Node::BlockBodies(lhs), Node::BlockBodies(rhs)) => {
                lhs.append(rhs);
            }
            (lhs, rhs) => *lhs = rhs.take(),
        }
    }
}

#[derive(Debug)]
pub struct BlockBuilder {
    name: String,
    labels: Vec<String>,
    body: Vec<Structure>,
}

impl BlockBuilder {
    pub fn new(name: &str) -> BlockBuilder {
        BlockBuilder {
            name: name.to_string(),
            labels: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn add_label(mut self, label: &str) -> BlockBuilder {
        self.labels.push(label.to_string());
        self
    }

    pub fn add_attribute(mut self, attr: Attribute) -> BlockBuilder {
        self.body.push(Structure::Attribute(attr));
        self
    }

    pub fn add_block(mut self, block: Block) -> BlockBuilder {
        self.body.push(Structure::Block(block));
        self
    }

    pub fn labels<I>(mut self, iter: I) -> BlockBuilder
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.labels = iter.into_iter().map(Into::into).collect();
        self
    }

    pub fn body<I>(mut self, iter: I) -> BlockBuilder
    where
        I: IntoIterator,
        I::Item: Into<Structure>,
    {
        self.body = iter.into_iter().map(Into::into).collect();
        self
    }

    pub fn build(self) -> Block {
        Block {
            name: self.name,
            labels: self.labels,
            body: Body::from_iter(self.body),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn test_into_value() {
        let body = Body::builder()
            .add_attribute(Attribute::new("foo", "bar"))
            .add_attribute(Attribute::new("bar", "baz"))
            .add_block(
                Block::builder("bar")
                    .add_label("baz")
                    .add_attribute(Attribute::new("foo", "bar"))
                    .build(),
            )
            .add_block(
                Block::builder("bar")
                    .add_label("qux")
                    .add_attribute(Attribute::new("foo", 1))
                    .build(),
            )
            .add_block(
                Block::builder("bar")
                    .add_label("baz")
                    .add_attribute(Attribute::new("bar", "baz"))
                    .build(),
            )
            .add_attribute(Attribute::new("foo", "baz"))
            .build();

        let value = json!({
            "foo": "baz",
            "bar": {
                "baz": [
                    {
                        "foo": "bar"
                    },
                    {
                        "bar": "baz"
                    }
                ],
                "qux": {
                    "foo": 1
                }
            }
        });

        let expected: Value = serde_json::from_value(value).unwrap();

        assert_eq!(Value::from(body), expected);
    }
}
