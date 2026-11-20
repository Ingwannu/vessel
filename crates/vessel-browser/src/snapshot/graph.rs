//! A flat tagged graph avoids attacker-selected recursive serde allocations.
use super::{MAX_SNAPSHOT_DEPTH, MAX_SNAPSHOT_NODES, SnapshotError};
use serde::de::{self, DeserializeSeed, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use vessel_proto::browser::{ElementRef, MAX_PAGE_TEXT_BYTES, PageText, SnapshotDocument};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Graph {
    pub(super) document: SnapshotDocument,
    pub(super) nodes: Nodes,
    pub(super) visited: u32,
    pub(super) frames: u32,
    pub(super) truncated: bool,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Node {
    Text {
        depth: u8,
        text: PageText,
    },
    Element {
        depth: u8,
        role: Role,
        name: PageText,
        reference: Option<ElementRef>,
        fingerprint: Fingerprint,
        state: State,
    },
}
impl Node {
    pub(super) const fn depth(&self) -> u8 {
        match self {
            Self::Text { depth, .. } | Self::Element { depth, .. } => *depth,
        }
    }
    fn bytes(&self) -> usize {
        match self {
            Self::Text { text, .. } => text.as_str().len(),
            Self::Element {
                name, fingerprint, ..
            } => name.as_str().len().saturating_add(fingerprint.bytes()),
        }
    }
}
/// State is a closed vocabulary; page attributes cannot inject a rendered YAML line.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct State {
    pub(super) checked: Option<Checked>,
    pub(super) expanded: Option<bool>,
    pub(super) selected: Option<bool>,
    pub(super) disabled: bool,
    pub(super) readonly: bool,
    pub(super) level: Option<HeadingLevel>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Checked {
    True,
    False,
    Mixed,
}
#[derive(Debug, Deserialize)]
#[serde(try_from = "u8")]
pub(super) struct HeadingLevel(u8);
impl TryFrom<u8> for HeadingLevel {
    type Error = &'static str;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if (1..=6).contains(&value) {
            Ok(Self(value))
        } else {
            Err("invalid snapshot heading level")
        }
    }
}
impl HeadingLevel {
    pub(super) const fn get(&self) -> u8 {
        self.0
    }
}

/// Exact state retained for later replacement detection; never shown in tool output/Debug.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Fingerprint {
    pub(super) tag: PageText,
    pub(super) id: PageText,
    pub(super) input_type: PageText,
    pub(super) name: PageText,
    pub(super) value: PageText,
}
impl Fingerprint {
    fn bytes(&self) -> usize {
        [
            &self.tag,
            &self.id,
            &self.input_type,
            &self.name,
            &self.value,
        ]
        .iter()
        .fold(0_usize, |size, field| {
            size.saturating_add(field.as_str().len())
        })
    }
}
impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SnapshotFingerprint([redacted])")
    }
}
macro_rules! roles {
    ($($role:ident => $name:literal),+ $(,)?) => {
        #[derive(Debug, Deserialize)]
        pub(super) enum Role { $(#[serde(rename=$name)] $role,)+ }
        impl Role {
            pub(super) const NAMES: &'static [&'static str] = &[$($name,)+];
            pub(super) const fn name(&self) -> &'static str { match self { $(Self::$role => $name,)+ } }
        }
    }
}
roles! {
    Document=>"document", Button=>"button", Link=>"link", Textbox=>"textbox",
    Searchbox=>"searchbox", Checkbox=>"checkbox", Radio=>"radio", Combobox=>"combobox",
    Listbox=>"listbox", Option=>"option", Slider=>"slider", Spinbutton=>"spinbutton",
    Switch=>"switch", Heading=>"heading", Image=>"img", Navigation=>"navigation",
    Main=>"main", Banner=>"banner", Contentinfo=>"contentinfo", Complementary=>"complementary",
    Form=>"form", Region=>"region", Article=>"article", List=>"list", Listitem=>"listitem",
    Table=>"table", Row=>"row", Cell=>"cell", Columnheader=>"columnheader", Rowheader=>"rowheader",
    Dialog=>"dialog", Alert=>"alert", Status=>"status", Tab=>"tab", Tablist=>"tablist",
    Tabpanel=>"tabpanel", Menu=>"menu", Menuitem=>"menuitem", Progressbar=>"progressbar",
    Separator=>"separator", Group=>"group", Tree=>"tree", Treeitem=>"treeitem", Iframe=>"iframe"
}
impl Graph {
    pub(super) fn validate(&self) -> Result<(), SnapshotError> {
        let visited = usize::try_from(self.visited).map_err(|_overflow| SnapshotError::Graph)?;
        if self.nodes.0.is_empty()
            || visited > MAX_SNAPSHOT_NODES
            || visited < self.nodes.0.len()
            || self.frames > self.visited
        {
            return Err(SnapshotError::Graph);
        }
        let mut previous = 0_u8;
        let mut bytes = self
            .document
            .address
            .as_str()
            .len()
            .saturating_add(self.document.title.as_str().len());
        for (index, node) in self.nodes.0.iter().enumerate() {
            let depth = node.depth();
            if depth > MAX_SNAPSHOT_DEPTH
                || (index == 0 && depth != 0)
                || depth > previous.saturating_add(1)
            {
                return Err(SnapshotError::Graph);
            }
            bytes = bytes.saturating_add(node.bytes());
            if bytes > MAX_PAGE_TEXT_BYTES {
                return Err(SnapshotError::Graph);
            }
            previous = depth;
        }
        Ok(())
    }
}
#[derive(Debug)]
pub(super) struct Nodes(pub(super) Vec<Node>);
impl<'de> Deserialize<'de> for Nodes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_seq(GraphVisitor).map(Self)
    }
}
struct GraphVisitor;
impl<'de> Visitor<'de> for GraphVisitor {
    type Value = Vec<Node>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a bounded flat browser snapshot")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut source: A) -> Result<Self::Value, A::Error> {
        let mut nodes = Vec::new();
        let mut bytes = 0_usize;
        while nodes.len() < MAX_SNAPSHOT_NODES {
            let Some(node) = source.next_element::<Node>()? else {
                return Ok(nodes);
            };
            bytes = bytes.saturating_add(node.bytes());
            if bytes > MAX_PAGE_TEXT_BYTES {
                return Err(de::Error::custom(
                    "snapshot graph text exceeds its byte budget",
                ));
            }
            nodes.push(node);
        }
        let _end = source.next_element_seed(RejectExcess)?;
        Ok(nodes)
    }
}
struct RejectExcess;
impl<'de> DeserializeSeed<'de> for RejectExcess {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _source: D) -> Result<(), D::Error> {
        Err(de::Error::custom("snapshot graph node limit exceeded"))
    }
}
