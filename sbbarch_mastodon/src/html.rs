use std::{fmt, fmt::Write, iter, rc::Rc};

use enumscribe::{ScribeStaticStr, TryUnscribe};
use libshire::{convert::Apply, either::Either::{self, Inl, Inr}};
use markup5ever_rcdom::{Node, NodeData};

#[derive(Debug)]
pub(crate) struct PostHtmlDoc {
    roots: Vec<PostHtmlNode>,
}

impl PostHtmlDoc {
    pub(crate) fn from_markup5ever_node(node: &Node, max_depth: usize) -> Option<Self> {
        let NodeData::Document = &node.data else {
            return None;
        };

        let roots = PostHtmlNode::conv_markup5ever_nodes_all(&*node.children.borrow(), max_depth);

        Some(Self { roots })
    }
}

impl fmt::Display for PostHtmlDoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for root in &self.roots {
            write!(f, "{}", root)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum PostHtmlNode {
    Element(PostHtmlElem),
    Text(String),
}

impl PostHtmlNode {
    fn conv_markup5ever_nodes_all(nodes: &[Rc<Node>], max_depth: usize) -> Vec<Self> {
        nodes
            .iter()
            .flat_map(|node| {
                Self::conv_markup5ever_node(node, max_depth)
                    .map_l(iter::once)
                    .into_iter()
                    .map(Either::fold_symmetric)
            })
            .apply(PostHtmlNodeIter::new)
            .collect()
    }

    fn conv_markup5ever_node(node: &Node, max_depth: usize) -> Either<Self, Vec<Self>> {
        let Some(max_depth) = max_depth.checked_sub(1) else {
            return Either::Inr(Vec::new());
        };

        match &node.data {
            NodeData::Document => Inr(Vec::new()),
            NodeData::Doctype {
                name: _,
                public_id: _,
                system_id: _,
            } => Inr(Vec::new()),
            NodeData::Text { contents } => Inl(Self::Text(contents.borrow().as_ref().to_owned())),
            NodeData::Comment { contents: _ } => Inr(Vec::new()),
            NodeData::Element {
                name,
                attrs,
                template_contents: _,
                mathml_annotation_xml_integration_point: _,
            } => {
                let children =
                    Self::conv_markup5ever_nodes_all(&*node.children.borrow(), max_depth);

                match PostHtmlTag::try_unscribe(&name.local) {
                    None => Inr(children),

                    Some(tag) => {
                        let attrs = attrs
                            .borrow()
                            .iter()
                            .filter_map(|attr| {
                                PostHtmlAttr::try_unscribe(&attr.name.local)
                                    .map(|name| (name, attr.value.as_ref().to_owned()))
                            })
                            .filter(|(name, _)| tag.is_attr_valid(*name))
                            .collect::<Vec<_>>();

                        Inl(PostHtmlNode::Element(PostHtmlElem {
                            tag,
                            attrs,
                            children,
                        }))
                    }
                }
            }
            NodeData::ProcessingInstruction {
                target: _,
                contents: _,
            } => Inr(Vec::new()),
        }
    }
}

impl fmt::Display for PostHtmlNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Element(elem) => fmt::Display::fmt(elem, f),
            Self::Text(text) => fmt::Display::fmt(&EscapeHtml(text), f),
        }
    }
}

struct PostHtmlNodeIter<I: Iterator<Item = PostHtmlNode>> {
    iter: I,
    peeked: Option<PostHtmlNode>,
}

impl<I: Iterator<Item = PostHtmlNode>> PostHtmlNodeIter<I> {
    fn new<J: IntoIterator<IntoIter = I>>(iter: J) -> Self {
        Self {
            iter: iter.into_iter(),
            peeked: None,
        }
    }
}

impl<I: Iterator<Item = PostHtmlNode>> Iterator for PostHtmlNodeIter<I> {
    type Item = PostHtmlNode;

    fn next(&mut self) -> Option<Self::Item> {
        match self.peeked.take().or_else(|| self.iter.next())? {
            elem @ PostHtmlNode::Element(_) => Some(elem),
            PostHtmlNode::Text(mut text) => {
                loop {
                    self.peeked = self.iter.next();
                    let Some(PostHtmlNode::Text(contiguous)) = &self.peeked else {
                        break;
                    };
                    text.push_str(contiguous);
                }
                Some(PostHtmlNode::Text(text))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

#[derive(Debug)]
pub(crate) struct PostHtmlElem {
    tag: PostHtmlTag,
    attrs: Vec<(PostHtmlAttr, String)>,
    children: Vec<PostHtmlNode>,
}

impl fmt::Display for PostHtmlElem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag_str = self.tag.scribe();
        write!(f, "<{}", tag_str)?;
        for (attr, val) in &self.attrs {
            write!(f, " {}='{}'", attr.scribe(), EscapeHtml(val))?;
        }
        write!(f, ">")?;
        if !self.children.is_empty() {
            for child in &self.children {
                write!(f, "{}", child)?;
            }
            write!(f, "</{}>", tag_str)?;
        }
        Ok(())
    }
}

#[derive(ScribeStaticStr, TryUnscribe, Clone, Copy, PartialEq, Eq, Debug)]
#[enumscribe(case_insensitive, rename_all = "lowercase")]
pub(crate) enum PostHtmlTag {
    P,
    Br,
    A,
    Del,
    Pre,
    Code,
    Em,
    Strong,
    B,
    I,
    U,
    Ul,
    Ol,
    Li,
    Blockquote,
}

impl PostHtmlTag {
    fn is_attr_valid(self, attr: PostHtmlAttr) -> bool {
        match (self, attr) {
            (Self::A, PostHtmlAttr::Href) => true,
            (Self::Ol, PostHtmlAttr::Start | PostHtmlAttr::Reversed) => true,
            (Self::Li, PostHtmlAttr::Value) => true,
            _ => false,
        }
    }
}

#[derive(ScribeStaticStr, TryUnscribe, Clone, Copy, PartialEq, Eq, Debug)]
#[enumscribe(case_insensitive, rename_all = "lowercase")]
enum PostHtmlAttr {
    Href,
    Start,
    Reversed,
    Value,
}

#[derive(Clone, Copy, Debug)]
struct EscapeHtml<'a>(pub &'a str);

impl<'a> fmt::Display for EscapeHtml<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            match escape_char(c) {
                Some(escaped) => f.write_str(escaped)?,
                None => f.write_char(c)?,
            }
        }
        Ok(())
    }
}

fn escape_char(c: char) -> Option<&'static str> {
    match c {
        '&' => Some("&amp;"),
        '\n' => Some("&nbsp;"),
        '"' => Some("&quot;"),
        '\'' => Some("&apos;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        _ => None,
    }
}
