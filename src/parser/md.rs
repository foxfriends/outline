//! The parser for Markdown based literate programming.
//!
//! This includes some extensions to support some of the more advanced features of this tool.
//!
//! See `examples/md/wc.c.md` for an example of how to use this format with the default config,
//! which is specified as follows:
//!
//! *   Code lines are enclosed in fenced code blocks, using `\`\`\`lang` as the fence.
//!
//!     The language tag is optional, but allows multiple languages to be used in one file. If
//!     there is no language specified, the language is inferred from the file type by using the
//!     second last extension.
//!
//!     When outputting the code, only blocks using the language from the file type will be
//!     printed. All languages will appear in the documentation.
//! *   A macro (named code block) separates the name from the language tag ` - `, such as
//!     `\`\`\`c - Name of the macro`. Note that the hyphen is surrounded by a single space on
//!     either side, even if there is no language tag.
//! *   The comment symbol is `//`, but they are still just rendered into the code
//!     *   Enabling the `comments_as_aside` option will render comments in an `<aside>` after the
//!         code block, which you may then style accordingly when the document is rendered.
//! *   Interpolation of is done such as `@{a meta variable}`.
//! *   Macros (named code blocks) are invoked by `==> Macro name.` (note the period at the end)
//!
//! As with all supported styles, all code blocks with the same name are concatenated, in the order
//! they are found, and the "unnamed" block is used as the entry point when generating the output
//! source file. Any code blocks with names which are not invoked will not appear in the compiled
//! code.
//!
//! Currently, the Markdown parser does not support code that is written to the compiled file, but
//! not rendered in the documentation file.


use std::iter::FromIterator;
use serde_derive::Deserialize;

use super::{Printer, Parser, ParserConfig, ParseError};

use crate::document::Document;
use crate::document::ast::Node;
use crate::document::code::CodeBlock;
use crate::document::text::TextBlock;
use crate::util::try_collect::TryCollectExt;

#[derive(Deserialize, Debug)]
pub struct MdParser {
    pub fence_sequence: String,
    pub block_name_start: String,
    pub block_name_end: Option<String>,
    pub default_language: Option<String>,
    pub comments_as_aside: bool,
    pub comment_start: String,
    pub interpolation_start: String,
    pub interpolation_end: String,
    pub macro_start: String,
    pub macro_end: String,
}

impl Default for MdParser {
    fn default() -> Self {
        Self {
            default_language: None,
            fence_sequence: String::from("```"),
            block_name_start: String::from(" - "),
            block_name_end: None,
            comments_as_aside: false,
            comment_start: String::from("//"),
            interpolation_start: String::from("@{"),
            interpolation_end: String::from("}"),
            macro_start: String::from("==> "),
            macro_end: String::from("."),
        }
    }
}

impl MdParser {
    pub fn for_language(language: String) -> Self {
        Self {
            default_language: Some(language),
            ..Self::default()
        }
    }
}

impl ParserConfig for MdParser {
    fn comment_start(&self) -> &str { &self.comment_start }
    fn interpolation_start(&self) -> &str { &self.interpolation_start }
    fn interpolation_end(&self) -> &str { &self.interpolation_end }
    fn macro_start(&self) -> &str { &self.macro_start }
    fn macro_end(&self) -> &str { &self.macro_end }
}

impl Parser for MdParser {
    type Error = MdError;

    fn parse<'a>(&self, input: &'a str) -> Result<Document<'a>, Self::Error> {
        #[derive(Default)]
        struct State<'a> {
            node: Option<Node<'a>>,
        }

        enum Parse<'a> {
            Incomplete,
            Complete(Node<'a>),
            Error(MdError),
        }

        let mut state = State::default();
        let mut document = input.lines()
            .enumerate()
            .scan(&mut state, |state, (line_number, line)| {
                if line.trim_start().starts_with(&self.fence_sequence) {
                    match state.node.take() {
                        Some(Node::Code(code_block)) => {
                            if line.starts_with(code_block.indent) {
                                state.node = None;
                                Some(Parse::Complete(Node::Code(code_block)))
                            } else {
                                Some(Parse::Error(MdError::Single {
                                    line_number,
                                    kind: MdErrorKind::IncorrectIndentation,
                                }))
                            }
                        }
                        previous => {
                            let indent_len = line.find(&self.fence_sequence).unwrap();
                            let (indent, rest) = line.split_at(indent_len);
                            let rest = &rest[self.fence_sequence.len()..];
                            let name_start = rest.find(&self.block_name_start);
                            let name = name_start
                                .map(|start| start + self.block_name_start.len())
                                .map(|name_start| {
                                    let name_end = self.block_name_end
                                        .as_ref()
                                        .and_then(|end| rest[name_start..].find(end))
                                        .map(|name_end| self.block_name_start.len() + name_end + name_start)
                                        .unwrap_or(rest.len());
                                    let name = &rest[name_start..name_end];
                                    self.parse_name(name)
                                });
                            let language = rest[..name_start.unwrap_or(rest.len())].trim();
                            let language = if language.is_empty() {
                                match &self.default_language {
                                    Some(language) => language.to_owned(),
                                    None => return Some(Parse::Error(MdError::Single {
                                        line_number,
                                        kind: MdErrorKind::UnknownLanguage,
                                    })),
                                }
                            } else {
                                language.to_owned()
                            };
                            let code_block = CodeBlock::new()
                                .indented(indent)
                                .in_language(language);
                            let code_block = match name {
                                None => code_block,
                                Some(Ok((name, vars))) => code_block.named(name, vars),
                                Some(Err(error)) => return Some(Parse::Error(MdError::Single {
                                    line_number,
                                    kind: error.into()
                                })),
                            };
                            state.node = Some(Node::Code(code_block));
                            match previous {
                                None => Some(Parse::Incomplete),
                                Some(node) => Some(Parse::Complete(node)),
                            }
                        }
                    }
                } else {
                    match &mut state.node {
                        None => {
                            let mut new_block = TextBlock::new();
                            new_block.add_line(line);
                            state.node = Some(Node::Text(new_block));
                            Some(Parse::Incomplete)
                        }
                        Some(Node::Text(block)) => {
                            block.add_line(line);
                            Some(Parse::Incomplete)
                        }
                        Some(Node::Code(block)) => {
                            if line.starts_with(block.indent) {
                                let line = match self.parse_line(line_number, &line[block.indent.len()..]) {
                                    Ok(line) => line,
                                    Err(error) => return Some(Parse::Error(MdError::Single {
                                        line_number,
                                        kind: error.into()
                                    })),
                                };
                                block.add_line(line);
                                Some(Parse::Incomplete)
                            } else {
                                Some(Parse::Error(MdError::Single {
                                    line_number,
                                    kind: MdErrorKind::IncorrectIndentation,
                                }))
                            }
                        }
                    }
                }
            })
            .filter_map(|parse| match parse {
                Parse::Incomplete => None,
                Parse::Error(error) => Some(Err(error)),
                Parse::Complete(node) => Some(Ok(node)),
            })
            .try_collect::<_, _, Vec<_>, MdError>()?;
        if let Some(node) = state.node.take() {
            document.push(node);
        }
        Ok(Document::from_iter(document))
    }
}

impl Printer for MdParser {
    fn print_text_block<'a>(&self, block: &TextBlock<'a>) -> String { format!("{}\n", block.to_string()) }

    fn print_code_block<'a>(&self, block: &CodeBlock<'a>) -> String {
        let mut output = self.fence_sequence.clone();
        if let Some(language) = &block.language {
            output.push_str(language);
        }
        if let Some(name) = &block.name {
            output.push_str(&self.block_name_start);
            output.push_str(&self.print_name(name.clone(), &block.vars));
            if let Some(end) = &self.block_name_end {
                output.push_str(end);
            }
        }
        output.push('\n');

        let mut comments = vec![];
        let line_offset = block.line_number().unwrap_or(0);
        for line in &block.source {
            output.push_str(&self.print_line(&line, !self.comments_as_aside));
            if self.comments_as_aside {
                if let Some(comment) = &line.comment {
                    comments.push((line.line_number - line_offset, comment));
                }
            }
            output.push('\n');
        }

        output.push_str(&self.fence_sequence);
        output.push('\n');

        for (line, comment) in comments {
            output.push_str(&format!("<aside class=\"comment\" data-line=\"{}\">{}</aside>\n", line, comment.trim()));
        }

        output
    }
}

#[derive(Debug)]
pub enum MdErrorKind {
    IncorrectIndentation,
    UnknownLanguage,
    Parse(ParseError),
}

#[derive(Debug)]
pub enum MdError {
    Single {
        line_number: usize,
        kind: MdErrorKind,
    },
    Multi(Vec<MdError>),
}

impl std::fmt::Display for MdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MdError::Multi(errors) => {
                for error in errors {
                    writeln!(f, "{}", error)?;
                }
                Ok(())
            }
            MdError::Single { line_number, kind } => writeln!(f, "{:?} (line {})", kind, line_number),
        }
    }
}

impl std::error::Error for MdError {}

impl FromIterator<MdError> for MdError {
    fn from_iter<I: IntoIterator<Item = MdError>>(iter: I) -> Self {
        MdError::Multi(iter.into_iter().collect())
    }
}

impl From<Vec<MdError>> for MdError {
    fn from(multi: Vec<MdError>) -> Self {
        MdError::Multi(multi)
    }
}

impl From<ParseError> for MdErrorKind {
    fn from(error: ParseError) -> Self {
        MdErrorKind::Parse(error)
    }
}