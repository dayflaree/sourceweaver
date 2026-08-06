use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Property { key: String, value: String },
    Block { name: String, body: Vec<Node> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.message, self.position)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Text(String, usize),
    Open(usize),
    Close(usize),
}

pub fn parse_document(input: &str) -> Result<Document, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens, cursor: 0 };
    let nodes = parser.parse_nodes(false)?;
    if parser.cursor != parser.tokens.len() {
        return Err(ParseError::new(
            "unexpected trailing token",
            parser.current_pos(),
        ));
    }
    Ok(Document { nodes })
}

impl Document {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        parse_document(input)
    }

    pub fn top_level_blocks(&self, name: &str) -> impl Iterator<Item = &Node> {
        self.nodes
            .iter()
            .filter(move |node| node.block_name() == Some(name))
    }

    pub fn top_level_blocks_mut(&mut self, name: &str) -> impl Iterator<Item = &mut Node> {
        self.nodes
            .iter_mut()
            .filter(move |node| node.block_name() == Some(name))
    }

    pub fn first_top_level_block_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.nodes
            .iter_mut()
            .find(|node| node.block_name() == Some(name))
    }

    pub fn to_vmf_string(&self) -> String {
        let mut out = String::new();
        for node in &self.nodes {
            write_node(node, 0, &mut out);
        }
        out
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_vmf_string())
    }
}

impl Node {
    pub fn block_name(&self) -> Option<&str> {
        match self {
            Node::Block { name, .. } => Some(name.as_str()),
            Node::Property { .. } => None,
        }
    }

    pub fn as_body(&self) -> Option<&[Node]> {
        match self {
            Node::Block { body, .. } => Some(body.as_slice()),
            Node::Property { .. } => None,
        }
    }

    pub fn as_body_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Node::Block { body, .. } => Some(body),
            Node::Property { .. } => None,
        }
    }

    pub fn property_key(&self) -> Option<&str> {
        match self {
            Node::Property { key, .. } => Some(key.as_str()),
            Node::Block { .. } => None,
        }
    }

    pub fn property_value(&self) -> Option<&str> {
        match self {
            Node::Property { value, .. } => Some(value.as_str()),
            Node::Block { .. } => None,
        }
    }

    pub fn get_property<'a>(body: &'a [Node], key: &str) -> Option<&'a str> {
        body.iter().find_map(|node| match node {
            Node::Property { key: k, value } if k == key => Some(value.as_str()),
            _ => None,
        })
    }

    pub fn set_property(body: &mut Vec<Node>, key: &str, value: impl Into<String>) {
        let value = value.into();
        for node in body.iter_mut() {
            if let Node::Property { key: k, value: v } = node {
                if k == key {
                    *v = value;
                    return;
                }
            }
        }
        body.insert(
            0,
            Node::Property {
                key: key.to_string(),
                value,
            },
        );
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b' ' | b'\t' | b'\r' | b'\n' => cursor += 1,
            b'/' if cursor + 1 < bytes.len() && bytes[cursor + 1] == b'/' => {
                cursor += 2;
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            b'{' => {
                tokens.push(Token::Open(cursor));
                cursor += 1;
            }
            b'}' => {
                tokens.push(Token::Close(cursor));
                cursor += 1;
            }
            b'"' => {
                let start = cursor;
                cursor += 1;
                let mut text = String::new();
                let mut closed = false;
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        b'"' => {
                            cursor += 1;
                            tokens.push(Token::Text(text, start));
                            closed = true;
                            break;
                        }
                        b'\\' if cursor + 1 < bytes.len() => {
                            cursor += 1;
                            text.push(bytes[cursor] as char);
                            cursor += 1;
                        }
                        other => {
                            text.push(other as char);
                            cursor += 1;
                        }
                    }
                }
                if !closed {
                    return Err(ParseError::new("unterminated quoted string", start));
                }
            }
            _ => {
                let start = cursor;
                let mut text = String::new();
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        b' ' | b'\t' | b'\r' | b'\n' | b'{' | b'}' | b'"' => break,
                        b'/' if cursor + 1 < bytes.len() && bytes[cursor + 1] == b'/' => break,
                        other => {
                            text.push(other as char);
                            cursor += 1;
                        }
                    }
                }
                if text.is_empty() {
                    return Err(ParseError::new("unexpected character", cursor));
                }
                tokens.push(Token::Text(text, start));
            }
        }
    }

    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn parse_nodes(&mut self, stop_on_close: bool) -> Result<Vec<Node>, ParseError> {
        let mut nodes = Vec::new();
        while self.cursor < self.tokens.len() {
            if matches!(self.tokens[self.cursor], Token::Close(_)) {
                if stop_on_close {
                    self.cursor += 1;
                    return Ok(nodes);
                }
                return Err(ParseError::new(
                    "unexpected closing brace",
                    self.current_pos(),
                ));
            }

            let (key, key_pos) = self.expect_text()?;
            match self.tokens.get(self.cursor) {
                Some(Token::Open(_)) => {
                    self.cursor += 1;
                    let body = self.parse_nodes(true)?;
                    nodes.push(Node::Block { name: key, body });
                }
                Some(Token::Text(_, _)) => {
                    let (value, _) = self.expect_text()?;
                    nodes.push(Node::Property { key, value });
                }
                Some(Token::Close(_)) | None => {
                    return Err(ParseError::new(
                        format!("expected value or block after `{key}`"),
                        key_pos,
                    ));
                }
            }
        }

        if stop_on_close {
            return Err(ParseError::new("missing closing brace", self.current_pos()));
        }

        Ok(nodes)
    }

    fn expect_text(&mut self) -> Result<(String, usize), ParseError> {
        match self.tokens.get(self.cursor) {
            Some(Token::Text(value, pos)) => {
                self.cursor += 1;
                Ok((value.clone(), *pos))
            }
            Some(Token::Open(pos)) => Err(ParseError::new("unexpected opening brace", *pos)),
            Some(Token::Close(pos)) => Err(ParseError::new("unexpected closing brace", *pos)),
            None => Err(ParseError::new(
                "unexpected end of file",
                self.current_pos(),
            )),
        }
    }

    fn current_pos(&self) -> usize {
        match self.tokens.get(self.cursor) {
            Some(Token::Text(_, pos)) | Some(Token::Open(pos)) | Some(Token::Close(pos)) => *pos,
            None => usize::MAX,
        }
    }
}

fn write_node(node: &Node, indent: usize, out: &mut String) {
    let tabs = "\t".repeat(indent);
    match node {
        Node::Property { key, value } => {
            out.push_str(&tabs);
            write_quoted(key, out);
            out.push(' ');
            write_quoted(value, out);
            out.push('\n');
        }
        Node::Block { name, body } => {
            out.push_str(&tabs);
            out.push_str(name);
            out.push('\n');
            out.push_str(&tabs);
            out.push_str("{\n");
            for child in body {
                write_node(child, indent + 1, out);
            }
            out.push_str(&tabs);
            out.push_str("}\n");
        }
    }
}

fn write_quoted(value: &str, out: &mut String) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_writes_basic_vmf() {
        let doc =
            parse_document("versioninfo { \"editorversion\" \"400\" }\nworld { \"id\" \"1\" }")
                .unwrap();
        assert_eq!(doc.nodes.len(), 2);
        assert!(doc.to_vmf_string().contains("versioninfo"));
        assert!(doc.to_vmf_string().contains("\"editorversion\" \"400\""));
    }

    #[test]
    fn ignores_line_comments() {
        let doc = parse_document("// hello\nworld { \"id\" \"1\" } // tail\n").unwrap();
        assert_eq!(doc.nodes.len(), 1);
    }
}
