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
                    map.insert(attr.key, Node::Value(attr.value));
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
    pub key: String,
    pub value: Value,
}

impl Attribute {
    pub fn new<K, V>(key: K, value: V) -> Attribute
    where
        K: Into<String>,
        V: Into<Value>,
    {
        Attribute {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl From<Attribute> for Value {
    fn from(attr: Attribute) -> Value {
        Value::from_iter(std::iter::once((attr.key, attr.value)))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub identifier: String,
    pub labels: Vec<BlockLabel>,
    pub body: Body,
}

impl Block {
    pub fn new<I, L>(identifier: I, labels: L, body: Body) -> Block
    where
        I: Into<String>,
        L: IntoIterator,
        L::Item: Into<BlockLabel>,
    {
        Block {
            identifier: identifier.into(),
            labels: labels.into_iter().map(Into::into).collect(),
            body,
        }
    }

    pub fn builder<I>(identifier: I) -> BlockBuilder
    where
        I: Into<String>,
    {
        BlockBuilder::new(identifier)
    }

    fn into_map(self) -> Map<String, Node> {
        let mut labels = self.labels.into_iter();

        let node = match labels.next() {
            Some(label) => {
                let block = Block {
                    identifier: label.into_inner(),
                    labels: labels.collect(),
                    body: self.body,
                };

                Node::Block(block.into_map())
            }
            None => Node::BlockBodies(vec![self.body]),
        };

        Map::from_iter(std::iter::once((self.identifier, node)))
    }
}

impl From<Block> for Value {
    fn from(block: Block) -> Value {
        Value::from_iter(block.into_map())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum BlockLabel {
    Identifier(String),
    StringLit(String),
}

impl BlockLabel {
    pub fn identifier<I>(identifier: I) -> Self
    where
        I: Into<String>,
    {
        BlockLabel::Identifier(identifier.into())
    }

    pub fn string_lit<S>(string: S) -> Self
    where
        S: Into<String>,
    {
        BlockLabel::StringLit(string.into())
    }

    pub fn into_inner(self) -> String {
        match self {
            BlockLabel::Identifier(ident) => ident,
            BlockLabel::StringLit(string) => string,
        }
    }
}

impl<T> From<T> for BlockLabel
where
    T: Into<String>,
{
    fn from(v: T) -> BlockLabel {
        BlockLabel::string_lit(v)
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
    identifier: String,
    labels: Vec<BlockLabel>,
    body: Vec<Structure>,
}

impl BlockBuilder {
    pub fn new<I>(identifier: I) -> BlockBuilder
    where
        I: Into<String>,
    {
        BlockBuilder {
            identifier: identifier.into(),
            labels: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn add_label<L>(mut self, label: L) -> BlockBuilder
    where
        L: Into<BlockLabel>,
    {
        self.labels.push(label.into());
        self
    }

    pub fn add_attribute<A>(mut self, attr: A) -> BlockBuilder
    where
        A: Into<Attribute>,
    {
        self.body.push(Structure::Attribute(attr.into()));
        self
    }

    pub fn add_block<B>(mut self, block: B) -> BlockBuilder
    where
        B: Into<Block>,
    {
        self.body.push(Structure::Block(block.into()));
        self
    }

    pub fn labels<L>(mut self, labels: L) -> BlockBuilder
    where
        L: IntoIterator,
        L::Item: Into<BlockLabel>,
    {
        self.labels = labels.into_iter().map(Into::into).collect();
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
            identifier: self.identifier,
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
