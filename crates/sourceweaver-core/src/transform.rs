use crate::vmf::{Document, Node};
use std::fmt;
use std::ops::{Add, Sub};

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Self {
            x: parts[0].parse().ok()?,
            y: parts[1].parse().ok()?,
            z: parts[2].parse().ok()?,
        })
    }

    pub fn to_vmf(self) -> String {
        format_number_triplet(self.x, self.y, self.z)
    }
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Self) -> Self::Output {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_vmf())
    }
}

pub fn find_landmark_origin(document: &Document, targetname: &str) -> Option<Vec3> {
    document.nodes.iter().find_map(|node| match node {
        Node::Block { name, body } if name == "entity" => {
            let classname = Node::get_property(body, "classname")?;
            if classname != "info_landmark" {
                return None;
            }
            let entity_targetname = Node::get_property(body, "targetname")?;
            if entity_targetname != targetname {
                return None;
            }
            Vec3::parse(Node::get_property(body, "origin")?)
        }
        _ => None,
    })
}

pub fn translate_block(node: &mut Node, offset: Vec3) {
    if offset == Vec3::ZERO {
        return;
    }

    if let Node::Block { body, .. } = node {
        translate_body(body, offset);
    }
}

pub fn translate_document(document: &mut Document, offset: Vec3) {
    for node in &mut document.nodes {
        translate_block(node, offset);
    }
}

fn translate_body(body: &mut Vec<Node>, offset: Vec3) {
    for node in body {
        match node {
            Node::Property { key, value } if key == "origin" => {
                if let Some(point) = Vec3::parse(value) {
                    *value = (point + offset).to_vmf();
                }
            }
            Node::Property { key, value } if key == "plane" => {
                if let Some(translated) = translate_plane(value, offset) {
                    *value = translated;
                }
            }
            Node::Property { key, value } if key == "startposition" => {
                if let Some(translated) = translate_wrapped_vec3(value, offset) {
                    *value = translated;
                }
            }
            Node::Block { body, .. } => translate_body(body, offset),
            _ => {}
        }
    }
}

fn translate_plane(value: &str, offset: Vec3) -> Option<String> {
    let points = parse_parenthesized_points(value)?;
    if points.len() != 3 {
        return None;
    }
    Some(
        points
            .into_iter()
            .map(|point| format!("({})", (point + offset).to_vmf()))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn translate_wrapped_vec3(value: &str, offset: Vec3) -> Option<String> {
    let trimmed = value.trim();
    if let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        return Some(format!("[{}]", (Vec3::parse(inner)? + offset).to_vmf()));
    }
    if let Some(inner) = trimmed
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return Some(format!("({})", (Vec3::parse(inner)? + offset).to_vmf()));
    }
    Vec3::parse(trimmed).map(|point| (point + offset).to_vmf())
}

fn parse_parenthesized_points(value: &str) -> Option<Vec<Vec3>> {
    let mut points = Vec::new();
    let mut rest = value;
    loop {
        let start = rest.find('(')?;
        let after_start = &rest[start + 1..];
        let end = after_start.find(')')?;
        points.push(Vec3::parse(&after_start[..end])?);
        rest = &after_start[end + 1..];
        if !rest.contains('(') {
            break;
        }
    }
    Some(points)
}

fn format_number_triplet(x: f64, y: f64, z: f64) -> String {
    format!(
        "{} {} {}",
        format_number(x),
        format_number(y),
        format_number(z)
    )
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < 0.000_001 {
        format!("{}", value.round() as i64)
    } else {
        let mut formatted = format!("{value:.6}");
        while formatted.contains('.') && formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_origin_and_plane() {
        let mut block = Node::Block {
            name: "entity".to_string(),
            body: vec![
                Node::Property {
                    key: "origin".into(),
                    value: "1 2 3".into(),
                },
                Node::Block {
                    name: "solid".into(),
                    body: vec![Node::Block {
                        name: "side".into(),
                        body: vec![Node::Property {
                            key: "plane".into(),
                            value: "(0 0 0) (1 0 0) (1 1 0)".into(),
                        }],
                    }],
                },
            ],
        };

        translate_block(&mut block, Vec3::new(10.0, 0.0, -3.0));
        let body = block.as_body().unwrap();
        assert_eq!(Node::get_property(body, "origin"), Some("11 2 0"));
    }

    #[test]
    fn translates_displacement_startposition_square_brackets() {
        let mut block = Node::Block {
            name: "side".to_string(),
            body: vec![Node::Block {
                name: "dispinfo".to_string(),
                body: vec![Node::Property {
                    key: "startposition".to_string(),
                    value: "[0 0 128]".to_string(),
                }],
            }],
        };

        translate_block(&mut block, Vec3::new(16.0, -8.0, 32.0));

        let dispinfo_body = match &block.as_body().unwrap()[0] {
            Node::Block { body, .. } => body,
            _ => panic!("expected dispinfo block"),
        };
        assert_eq!(
            Node::get_property(dispinfo_body, "startposition"),
            Some("[16 -8 160]")
        );
    }

    #[test]
    fn translates_displacement_startposition_parentheses_and_bare_vectors() {
        assert_eq!(
            translate_wrapped_vec3("(1 2 3)", Vec3::new(10.0, 0.0, -3.0)),
            Some("(11 2 0)".to_string())
        );
        assert_eq!(
            translate_wrapped_vec3("1 2 3", Vec3::new(10.0, 0.0, -3.0)),
            Some("11 2 0".to_string())
        );
    }
}
