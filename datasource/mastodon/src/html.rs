use std::{fmt, fmt::Write, iter, rc::Rc};

use enumscribe::{ScribeStaticStr, TryUnscribe};
use html5ever::{
  interface::QuirksMode, local_name, namespace_url, ns, parse_fragment, tendril::TendrilSink,
  tokenizer::TokenizerOpts, tree_builder::TreeBuilderOpts, ParseOpts, QualName,
};
use libshire::{
  convert::Apply,
  either::Either::{self, Inl, Inr},
};
use markup5ever_rcdom::{Node, NodeData, RcDom};

#[derive(Clone, Debug)]
pub struct MdonHtmlDoc {
  roots: Vec<MdonHtmlNode>,
}

impl MdonHtmlDoc {
  pub fn from_roots(roots: Vec<MdonHtmlNode>) -> Self {
    Self { roots }
  }

  pub fn from_html_str(html: &str, max_depth: usize) -> Option<Self> {
    let dom = parse_fragment(
      RcDom::default(),
      Self::html_parse_opts(),
      QualName::new(None, ns!(html), local_name!("body")),
      Vec::new(),
    )
    .one(html);

    Self::from_markup5ever_node(&dom.document, max_depth)
  }

  pub fn from_markup5ever_node(node: &Node, max_depth: usize) -> Option<Self> {
    let NodeData::Document = &node.data else {
      return None;
    };

    let roots = MdonHtmlNode::conv_markup5ever_nodes_all(&*node.children.borrow(), max_depth);

    Some(Self::from_roots(roots))
  }

  pub fn roots(&self) -> &[MdonHtmlNode] {
    &self.roots
  }

  fn html_parse_opts() -> ParseOpts {
    ParseOpts {
      tokenizer: TokenizerOpts::default(),

      tree_builder: TreeBuilderOpts {
        exact_errors: false,
        scripting_enabled: false,
        iframe_srcdoc: false,
        drop_doctype: true,
        ignore_missing_rules: false,
        quirks_mode: QuirksMode::NoQuirks,
      },
    }
  }
}

impl fmt::Display for MdonHtmlDoc {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for root in &self.roots {
      write!(f, "{}", root)?;
    }
    Ok(())
  }
}

#[derive(Clone, Debug)]
pub enum MdonHtmlNode {
  Element(MdonHtmlElem),
  Text(String),
}

impl MdonHtmlNode {
  fn conv_markup5ever_nodes_all(nodes: &[Rc<Node>], max_depth: usize) -> Vec<Self> {
    nodes
      .iter()
      .flat_map(|node| {
        Self::conv_markup5ever_node(node, max_depth)
          .map_l(iter::once)
          .into_iter()
          .map(Either::fold_symmetric)
      })
      .apply(MdonHtmlNodeIter::new)
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
        let children = Self::conv_markup5ever_nodes_all(&*node.children.borrow(), max_depth);

        match MdonHtmlTag::try_unscribe(&name.local) {
          None => Inr(children),

          Some(tag) => {
            let attrs = attrs.borrow();
            let attrs = attrs.iter().filter_map(|attr| {
              MdonHtmlAttr::try_unscribe(&attr.name.local)
                .map(|name| (name, attr.value.as_ref().to_owned()))
            });

            Inl(MdonHtmlNode::Element(MdonHtmlElem::new(
              tag, attrs, children,
            )))
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

impl fmt::Display for MdonHtmlNode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Element(elem) => fmt::Display::fmt(elem, f),
      Self::Text(text) => fmt::Display::fmt(&EscapeHtml(text), f),
    }
  }
}

struct MdonHtmlNodeIter<I: Iterator<Item = MdonHtmlNode>> {
  iter: I,
  peeked: Option<MdonHtmlNode>,
}

impl<I: Iterator<Item = MdonHtmlNode>> MdonHtmlNodeIter<I> {
  fn new<J: IntoIterator<IntoIter = I>>(iter: J) -> Self {
    Self {
      iter: iter.into_iter(),
      peeked: None,
    }
  }
}

impl<I: Iterator<Item = MdonHtmlNode>> Iterator for MdonHtmlNodeIter<I> {
  type Item = MdonHtmlNode;

  fn next(&mut self) -> Option<Self::Item> {
    match self.peeked.take().or_else(|| self.iter.next())? {
      elem @ MdonHtmlNode::Element(_) => Some(elem),
      MdonHtmlNode::Text(mut text) => {
        loop {
          self.peeked = self.iter.next();
          let Some(MdonHtmlNode::Text(contiguous)) = &self.peeked else {
            break;
          };
          text.push_str(contiguous);
        }
        Some(MdonHtmlNode::Text(text))
      }
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    self.iter.size_hint()
  }
}

#[derive(Clone, Debug)]
pub struct MdonHtmlElem {
  tag: MdonHtmlTag,
  attrs: Vec<(MdonHtmlAttr, String)>,
  children: Vec<MdonHtmlNode>,
}

impl MdonHtmlElem {
  pub fn new<Attrs>(tag: MdonHtmlTag, attrs: Attrs, children: Vec<MdonHtmlNode>) -> Self
  where
    Attrs: IntoIterator<Item = (MdonHtmlAttr, String)>,
  {
    let attrs = attrs
      .into_iter()
      .filter(|(attr, _)| tag.is_attr_valid(*attr))
      .collect::<Vec<_>>();

    Self {
      tag,
      attrs,
      children,
    }
  }

  pub fn clone_replace_children(&self, children: Vec<MdonHtmlNode>) -> Self {
    Self {
      tag: self.tag.clone(),
      attrs: self.attrs.clone(),
      children,
    }
  }

  pub fn tag(&self) -> MdonHtmlTag {
    self.tag
  }

  pub fn attrs(&self) -> &[(MdonHtmlAttr, String)] {
    &self.attrs
  }

  pub fn children(&self) -> &[MdonHtmlNode] {
    &self.children
  }
}

impl fmt::Display for MdonHtmlElem {
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
pub enum MdonHtmlTag {
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

impl MdonHtmlTag {
  fn is_attr_valid(self, attr: MdonHtmlAttr) -> bool {
    use {MdonHtmlAttr::*, MdonHtmlTag::*};
    match (self, attr) {
      (A, Href) => true,
      (Ol, Start | Reversed) => true,
      (Li, Value) => true,
      _ => false,
    }
  }
}

#[derive(ScribeStaticStr, TryUnscribe, Clone, Copy, PartialEq, Eq, Debug)]
#[enumscribe(case_insensitive, rename_all = "lowercase")]
pub enum MdonHtmlAttr {
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
