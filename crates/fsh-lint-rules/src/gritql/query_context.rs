//! QueryContext implementation for FSH
//!
//! This bridges our Rowan CST with grit-pattern-matcher's execution engine.

use super::cst_adapter::FshGritNode;
use super::cst_language::FshTargetLanguage;
use super::cst_tree::FshGritTree;
use grit_pattern_matcher::binding::Binding;
use grit_pattern_matcher::constant::Constant;
use grit_pattern_matcher::context::{ExecContext as GritExecContext, QueryContext};
use grit_pattern_matcher::effects::Effect;
use grit_pattern_matcher::file_owners::FileOwners;
use grit_pattern_matcher::pattern::{
    Accessor, AstLeafNodePattern, AstNodePattern, CallBuiltIn, CodeSnippet, DynamicPattern,
    DynamicSnippet, File, FilePtr, FileRegistry, GritFunctionDefinition, ListIndex, Pattern,
    PatternDefinition, PredicateDefinition, ResolvedFile, ResolvedPattern, ResolvedSnippet, State,
};
use grit_util::error::{GritPatternError, GritResult};
use grit_util::{AnalysisLogs, Ast, AstNode, ByteRange, CodeRange, Range};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct FshQueryContext;

impl QueryContext for FshQueryContext {
    type Node<'a> = FshGritNode;
    type NodePattern = FshNodePattern;
    type LeafNodePattern = FshLeafNodePattern;
    type ExecContext<'a> = FshExecContext<'a>;
    type Binding<'a> = FshBinding<'a>;
    type CodeSnippet = FshCodeSnippet;
    type ResolvedPattern<'a> = FshResolvedPattern<'a>;
    type Language<'a> = FshTargetLanguage;
    type File<'a> = FshFile<'a>;
    type Tree<'a> = FshGritTree;
}

#[derive(Clone, Debug)]
pub struct FshNodePattern {
    pub kind: fsh_lint_core::cst::FshSyntaxKind,
    pub args: Vec<FshNodePatternArg>,
}

#[derive(Clone, Debug)]
pub struct FshNodePatternArg {
    pub pattern: Pattern<FshQueryContext>,
    pub slot_index: u32,
}

#[derive(Clone, Debug)]
pub struct FshLeafNodePattern {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FshBinding<'a> {
    Node(FshGritNode),
    Range(ByteRange, &'a str),
    File(&'a Path),
    Constant(&'a Constant),
    Empty(FshGritNode, u32),
}

#[derive(Clone, Debug)]
pub struct FshCodeSnippet {
    pub patterns: Vec<(fsh_lint_core::cst::FshSyntaxKind, Pattern<FshQueryContext>)>,
    pub source: String,
    pub dynamic_snippet: Option<DynamicPattern<FshQueryContext>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FshResolvedPattern<'a> {
    Binding(Vec<FshBinding<'a>>),
    Snippets(Vec<ResolvedSnippet<'a, FshQueryContext>>),
    List(Vec<FshResolvedPattern<'a>>),
    Map(BTreeMap<String, FshResolvedPattern<'a>>),
    File(FshFile<'a>),
    Files(Box<FshResolvedPattern<'a>>),
    Constant(Constant),
}

impl<'a> FshResolvedPattern<'a> {
    pub fn from_node_binding(node: FshGritNode) -> Self {
        Self::Binding(vec![FshBinding::Node(node)])
    }

    pub fn from_constant_binding(constant: &'a Constant) -> Self {
        Self::Binding(vec![FshBinding::Constant(constant)])
    }
}

impl<'a> ResolvedPattern<'a, FshQueryContext> for FshResolvedPattern<'a> {
    fn from_binding(binding: FshBinding<'a>) -> Self {
        Self::Binding(vec![binding])
    }

    fn from_constant(constant: Constant) -> Self {
        Self::Constant(constant)
    }

    fn from_file_pointer(file: FilePtr) -> Self {
        Self::File(FshFile::Ptr(file))
    }

    fn from_files(files: Self) -> Self {
        Self::Files(Box::new(files))
    }

    fn from_list_parts(parts: impl Iterator<Item = Self>) -> Self {
        Self::List(parts.collect())
    }

    fn from_string(string: String) -> Self {
        Self::Snippets(vec![ResolvedSnippet::Text(string.into())])
    }

    fn from_resolved_snippet(snippet: ResolvedSnippet<'a, FshQueryContext>) -> Self {
        Self::Snippets(vec![snippet])
    }

    fn from_dynamic_snippet(
        _snippet: &'a DynamicSnippet,
        _state: &mut State<'a, FshQueryContext>,
        _context: &'a FshExecContext,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<Self> {
        Err(GritPatternError::new(
            "Dynamic snippets not yet implemented",
        ))
    }

    fn from_dynamic_pattern(
        pattern: &'a DynamicPattern<FshQueryContext>,
        state: &mut State<'a, FshQueryContext>,
        context: &'a FshExecContext,
        logs: &mut AnalysisLogs,
    ) -> GritResult<Self> {
        match pattern {
            DynamicPattern::Variable(var) => {
                let scope_idx = var.try_scope()?;
                let var_idx = var.try_index()?;
                let content = &state.bindings[scope_idx as usize].last().unwrap()[var_idx as usize];
                if let Some(value) = &content.value {
                    Ok(value.clone())
                } else if let Some(pattern) = content.pattern {
                    Self::from_pattern(pattern, state, context, logs)
                } else {
                    Err(GritPatternError::new("Unresolved variable"))
                }
            }
            DynamicPattern::Accessor(accessor) => {
                Self::from_accessor(accessor, state, context, logs)
            }
            DynamicPattern::ListIndex(index) => Self::from_list_index(index, state, context, logs),
            _ => Err(GritPatternError::new("Unsupported dynamic pattern")),
        }
    }

    fn from_accessor(
        _accessor: &'a Accessor<FshQueryContext>,
        _state: &mut State<'a, FshQueryContext>,
        _context: &'a FshExecContext,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<Self> {
        Err(GritPatternError::new("Accessors not yet implemented"))
    }

    fn from_list_index(
        _index: &'a ListIndex<FshQueryContext>,
        _state: &mut State<'a, FshQueryContext>,
        _context: &'a FshExecContext,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<Self> {
        Err(GritPatternError::new("List indexing not yet implemented"))
    }

    fn from_pattern(
        pattern: &'a Pattern<FshQueryContext>,
        state: &mut State<'a, FshQueryContext>,
        context: &'a FshExecContext,
        logs: &mut AnalysisLogs,
    ) -> GritResult<Self> {
        match pattern {
            Pattern::Dynamic(p) => Self::from_dynamic_pattern(p, state, context, logs),
            Pattern::StringConstant(s) => Ok(Self::from_string(s.text.clone())),
            Pattern::IntConstant(i) => Ok(Self::Constant(Constant::Integer(i.value))),
            Pattern::FloatConstant(f) => Ok(Self::Constant(Constant::Float(f.value))),
            Pattern::BooleanConstant(b) => Ok(Self::Constant(Constant::Boolean(b.value))),
            Pattern::Variable(var) => {
                let scope_idx = var.try_scope()?;
                let var_idx = var.try_index()?;
                let content = &state.bindings[scope_idx as usize].last().unwrap()[var_idx as usize];
                if let Some(value) = &content.value {
                    Ok(value.clone())
                } else if let Some(pattern) = content.pattern {
                    Self::from_pattern(pattern, state, context, logs)
                } else {
                    Err(GritPatternError::new("Unresolved variable"))
                }
            }
            _ => Err(GritPatternError::new("Cannot resolve this pattern type")),
        }
    }

    fn extend(
        &mut self,
        _with: Self,
        _effects: &mut Vec<Effect<'a, FshQueryContext>>,
        _language: &FshTargetLanguage,
    ) -> GritResult<()> {
        Err(GritPatternError::new("Rewriting not implemented"))
    }

    fn float(
        &self,
        _state: &FileRegistry<'a, FshQueryContext>,
        _language: &FshTargetLanguage,
    ) -> GritResult<f64> {
        match self {
            Self::Constant(Constant::Float(f)) => Ok(*f),
            Self::Constant(Constant::Integer(i)) => Ok(*i as f64),
            _ => Err(GritPatternError::new("Cannot convert to float")),
        }
    }

    fn get_bindings(&self) -> Option<impl Iterator<Item = FshBinding<'a>>> {
        if let Self::Binding(bindings) = self {
            Some(bindings.iter().cloned())
        } else {
            None
        }
    }

    fn get_file(&self) -> Option<&FshFile<'a>> {
        if let Self::File(file) = self {
            Some(file)
        } else {
            None
        }
    }

    fn get_file_pointers(&self) -> Option<Vec<FilePtr>> {
        match self {
            Self::File(FshFile::Ptr(ptr)) => Some(vec![*ptr]),
            Self::Files(files) => files.get_file_pointers(),
            _ => None,
        }
    }

    fn get_files(&self) -> Option<&Self> {
        if let Self::Files(files) = self {
            Some(files)
        } else {
            None
        }
    }

    fn get_last_binding(&self) -> Option<&FshBinding<'a>> {
        if let Self::Binding(bindings) = self {
            bindings.last()
        } else {
            None
        }
    }

    fn get_list_item_at(&self, index: isize) -> Option<&Self> {
        if let Self::List(items) = self {
            if index >= 0 && (index as usize) < items.len() {
                items.get(index as usize)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_list_item_at_mut(&mut self, index: isize) -> Option<&mut Self> {
        if let Self::List(items) = self {
            if index >= 0 && (index as usize) < items.len() {
                items.get_mut(index as usize)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_list_items(&self) -> Option<impl Iterator<Item = &Self>> {
        if let Self::List(items) = self {
            Some(items.iter())
        } else {
            None
        }
    }

    fn get_list_binding_items(&self) -> Option<impl Iterator<Item = Self> + Clone> {
        self.get_last_binding()
            .and_then(|b| b.list_items())
            .map(|items| items.map(FshResolvedPattern::from_node_binding))
    }

    fn get_map(&self) -> Option<&BTreeMap<String, Self>> {
        if let Self::Map(map) = self {
            Some(map)
        } else {
            None
        }
    }

    fn get_map_mut(&mut self) -> Option<&mut BTreeMap<String, Self>> {
        if let Self::Map(map) = self {
            Some(map)
        } else {
            None
        }
    }

    fn get_snippets(&self) -> Option<impl Iterator<Item = ResolvedSnippet<'a, FshQueryContext>>> {
        if let Self::Snippets(snippets) = self {
            Some(snippets.iter().cloned())
        } else {
            None
        }
    }

    fn is_binding(&self) -> bool {
        matches!(self, Self::Binding(_))
    }

    fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    fn is_truthy(
        &self,
        _state: &mut State<'a, FshQueryContext>,
        _language: &FshTargetLanguage,
    ) -> GritResult<bool> {
        Ok(match self {
            Self::Binding(bindings) => bindings.last().is_some_and(|b| b.is_truthy()),
            Self::List(elements) => !elements.is_empty(),
            Self::Map(map) => !map.is_empty(),
            Self::Constant(c) => c.is_truthy(),
            Self::Snippets(s) => !s.is_empty(),
            Self::File(_) | Self::Files(_) => true,
        })
    }

    fn linearized_text(
        &self,
        _language: &FshTargetLanguage,
        _effects: &[Effect<'a, FshQueryContext>],
        _files: &FileRegistry<'a, FshQueryContext>,
        _memo: &mut HashMap<CodeRange, Option<String>>,
        _should_pad_snippet: bool,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<Cow<'a, str>> {
        Err(GritPatternError::new("Rewriting not implemented"))
    }

    fn matches_undefined(&self) -> bool {
        matches!(self, Self::Constant(Constant::Undefined))
    }

    fn matches_false_or_undefined(&self) -> bool {
        matches!(
            self,
            Self::Constant(Constant::Boolean(false)) | Self::Constant(Constant::Undefined)
        )
    }

    fn normalize_insert(
        &mut self,
        _binding: &FshBinding,
        _is_first: bool,
        _language: &FshTargetLanguage,
    ) -> GritResult<()> {
        Err(GritPatternError::new("Insertion padding not implemented"))
    }

    fn position(&self, language: &FshTargetLanguage) -> Option<Range> {
        if let Self::Binding(bindings) = self {
            bindings.last().and_then(|b| b.position(language))
        } else {
            None
        }
    }

    fn push_binding(&mut self, binding: FshBinding<'a>) -> GritResult<()> {
        let Self::Binding(bindings) = self else {
            return Err(GritPatternError::new("can only push to bindings"));
        };
        bindings.push(binding);
        Ok(())
    }

    fn set_list_item_at_mut(&mut self, index: isize, value: Self) -> GritResult<bool> {
        let Self::List(items) = self else {
            return Err(GritPatternError::new("can only set items on a list"));
        };
        if index >= 0 && (index as usize) < items.len() {
            items.insert(index as usize, value);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn text(
        &self,
        files: &FileRegistry<'a, FshQueryContext>,
        lang: &FshTargetLanguage,
    ) -> GritResult<Cow<'a, str>> {
        match self {
            Self::Binding(bindings) => {
                let binding = bindings
                    .last()
                    .ok_or_else(|| GritPatternError::new("cannot get text of empty binding"))?;
                let text = binding.text(lang)?;
                Ok(Cow::Owned(text.into_owned()))
            }
            Self::Snippets(snippets) => {
                let text = snippets
                    .iter()
                    .map(|s| s.text(files, lang))
                    .collect::<GritResult<Vec<_>>>()?
                    .join("");
                Ok(Cow::Owned(text))
            }
            Self::Constant(c) => Ok(Cow::Owned(c.to_string())),
            _ => Ok(Cow::Owned("".to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FshFile<'a> {
    Ptr(FilePtr),
    Resolved(Box<ResolvedFile<'a, FshQueryContext>>),
}

impl<'a> FshBinding<'a> {
    pub fn list_items(&self) -> Option<impl Iterator<Item = FshGritNode> + Clone> {
        if let Self::Node(node) = self {
            if node.is_list() {
                return Some(node.named_children());
            }
        }
        None
    }
}

impl<'a> Binding<'a, FshQueryContext> for FshBinding<'a> {
    fn from_constant(constant: &'a Constant) -> Self {
        Self::Constant(constant)
    }

    fn from_node(node: FshGritNode) -> Self {
        Self::Node(node)
    }

    fn from_path(path: &'a Path) -> Self {
        Self::File(path)
    }

    fn from_range(range: ByteRange, source: &'a str) -> Self {
        Self::Range(range, source)
    }

    fn singleton(&self) -> Option<FshGritNode> {
        match self {
            Self::Node(node) => {
                if node.is_list() {
                    let mut children = node.named_children();
                    match (children.next(), children.next()) {
                        (Some(only_child), None) => Some(only_child),
                        _ => None,
                    }
                } else {
                    Some(node.clone())
                }
            }
            _ => None,
        }
    }

    fn get_sexp(&self) -> Option<String> {
        Some(match self {
            Self::Node(node) => format!("({node:?})"),
            Self::Range(range, _) => format!("(range {}-{})", range.start, range.end),
            Self::File(path) => format!("({})", path.display()),
            Self::Constant(c) => format!("({c})"),
            Self::Empty(_, _) => "(empty)".to_owned(),
        })
    }

    fn position(&self, _language: &FshTargetLanguage) -> Option<Range> {
        match self {
            Self::Node(node) => {
                let range = node.byte_range();
                Some(Range {
                    start: grit_util::Position {
                        line: 0,
                        column: range.start as u32,
                    },
                    end: grit_util::Position {
                        line: 0,
                        column: range.end as u32,
                    },
                    start_byte: range.start as u32,
                    end_byte: range.end as u32,
                })
            }
            Self::Range(range, _) => Some(Range {
                start: grit_util::Position {
                    line: 0,
                    column: range.start as u32,
                },
                end: grit_util::Position {
                    line: 0,
                    column: range.end as u32,
                },
                start_byte: range.start as u32,
                end_byte: range.end as u32,
            }),
            _ => None,
        }
    }

    fn range(&self, _language: &FshTargetLanguage) -> Option<ByteRange> {
        match self {
            Self::Node(node) => Some(node.byte_range()),
            Self::Range(range, _) => Some(*range),
            _ => None,
        }
    }

    fn code_range(&self, _language: &FshTargetLanguage) -> Option<CodeRange> {
        match self {
            Self::Node(node) => Some(node.code_range()),
            Self::Range(range, _) => Some(CodeRange::new(range.start as u32, range.end as u32, "")),
            _ => None,
        }
    }

    fn is_equivalent_to(&self, other: &Self, language: &FshTargetLanguage) -> bool {
        match (self, other) {
            (Self::Node(n1), Self::Node(n2)) => n1 == n2,
            (Self::Constant(c1), Self::Constant(c2)) => c1 == c2,
            (Self::File(f1), Self::File(f2)) => f1 == f2,
            _ => self.text(language).ok() == other.text(language).ok(),
        }
    }

    fn is_suppressed(&self, _language: &FshTargetLanguage, _current_name: Option<&str>) -> bool {
        false
    }

    fn get_insertion_padding(
        &self,
        _text: &str,
        _is_first: bool,
        _language: &FshTargetLanguage,
    ) -> Option<String> {
        None
    }

    fn linearized_text(
        &self,
        _language: &FshTargetLanguage,
        _effects: &[Effect<'a, FshQueryContext>],
        _files: &FileRegistry<'a, FshQueryContext>,
        _memo: &mut HashMap<CodeRange, Option<String>>,
        _distributed_indent: Option<usize>,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<Cow<'a, str>> {
        Err(GritPatternError::new("Rewriting not implemented"))
    }

    fn text(&self, _language: &FshTargetLanguage) -> GritResult<Cow<'a, str>> {
        match self {
            Self::Node(node) => Ok(Cow::Owned(node.text_content())),
            Self::Range(range, source) => {
                let text = &source[range.start..range.end];
                Ok(Cow::Borrowed(text))
            }
            Self::File(path) => Ok(path.to_string_lossy()),
            Self::Constant(c) => Ok(Cow::Owned(c.to_string())),
            Self::Empty(_, _) => Ok(Cow::Borrowed("")),
        }
    }

    fn source(&self) -> Option<&'a str> {
        match self {
            Self::Node(_node) => None, // Node source requires proper lifetime management
            Self::Range(_, source) => Some(source),
            Self::Empty(_node, _) => None, // Node source requires proper lifetime management
            Self::File(_) | Self::Constant(_) => None,
        }
    }

    fn as_constant(&self) -> Option<&Constant> {
        match self {
            Self::Constant(c) => Some(c),
            _ => None,
        }
    }

    fn as_filename(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            _ => None,
        }
    }

    fn as_node(&self) -> Option<FshGritNode> {
        match self {
            Self::Node(node) => Some(node.clone()),
            _ => None,
        }
    }

    fn is_list(&self) -> bool {
        matches!(self, Self::Node(node) if node.is_list())
    }

    fn is_truthy(&self) -> bool {
        match self {
            Self::Constant(c) => c.is_truthy(),
            Self::Empty(_, _) => false,
            _ => true,
        }
    }

    fn list_items(&self) -> Option<impl Iterator<Item = FshGritNode> + Clone> {
        self.list_items()
    }

    fn parent_node(&self) -> Option<FshGritNode> {
        match self {
            Self::Node(node) => node.parent(),
            _ => None,
        }
    }

    fn log_empty_field_rewrite_error(
        &self,
        _language: &FshTargetLanguage,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<()> {
        Ok(())
    }
}

impl<'a> FshResolvedPattern<'a> {
    pub fn from_path_binding(path: &'a Path) -> Self {
        Self::Binding(vec![FshBinding::File(path)])
    }
}

impl<'a> File<'a, FshQueryContext> for FshFile<'a> {
    fn name(&self, files: &FileRegistry<'a, FshQueryContext>) -> FshResolvedPattern<'a> {
        match self {
            Self::Ptr(ptr) => FshResolvedPattern::from_path_binding(files.get_file_name(*ptr)),
            Self::Resolved(resolved) => resolved.name.clone(),
        }
    }

    fn absolute_path(
        &self,
        files: &FileRegistry<'a, FshQueryContext>,
        _lang: &FshTargetLanguage,
    ) -> GritResult<FshResolvedPattern<'a>> {
        match self {
            Self::Ptr(ptr) => Ok(FshResolvedPattern::from_path_binding(
                files.get_absolute_path(*ptr)?,
            )),
            Self::Resolved(resolved) => {
                let name = resolved.name.text(files, _lang)?;
                Ok(FshResolvedPattern::Constant(Constant::String(
                    name.to_string(),
                )))
            }
        }
    }

    fn binding(&self, files: &FileRegistry<'a, FshQueryContext>) -> FshResolvedPattern<'a> {
        match self {
            Self::Ptr(ptr) => {
                let file = files.get_file_owner(*ptr);
                FshResolvedPattern::from_node_binding(file.tree.root_node())
            }
            Self::Resolved(resolved) => resolved.body.clone(),
        }
    }

    fn body(&self, files: &FileRegistry<'a, FshQueryContext>) -> FshResolvedPattern<'a> {
        match self {
            Self::Ptr(ptr) => {
                let file = files.get_file_owner(*ptr);
                FshResolvedPattern::from_tree(&file.tree)
            }
            Self::Resolved(resolved) => resolved.body.clone(),
        }
    }
}

impl<'a> FshResolvedPattern<'a> {
    pub fn from_tree(tree: &'a FshGritTree) -> Self {
        Self::from_node_binding(tree.root_node())
    }

    pub fn from_range_binding(range: ByteRange, source: &'a str) -> Self {
        Self::Binding(vec![FshBinding::Range(range, source)])
    }
}

impl CodeSnippet<FshQueryContext> for FshCodeSnippet {
    fn patterns(&self) -> impl Iterator<Item = &Pattern<FshQueryContext>> {
        self.patterns.iter().map(|p| &p.1)
    }

    fn dynamic_snippet(&self) -> Option<&DynamicPattern<FshQueryContext>> {
        self.dynamic_snippet.as_ref()
    }
}

use grit_pattern_matcher::context::StaticDefinitions;
use grit_pattern_matcher::pattern::{Matcher, PatternName, PatternOrPredicate};

impl AstNodePattern<FshQueryContext> for FshNodePattern {
    const INCLUDES_TRIVIA: bool = false;

    fn children(
        &self,
        _definitions: &StaticDefinitions<FshQueryContext>,
    ) -> Vec<PatternOrPredicate<'_, FshQueryContext>> {
        self.args
            .iter()
            .map(|arg| PatternOrPredicate::Pattern(&arg.pattern))
            .collect()
    }

    fn matches_kind_of(&self, node: &FshGritNode) -> bool {
        node.kind() == self.kind
    }
}

impl Matcher<FshQueryContext> for FshNodePattern {
    fn execute<'a>(
        &'a self,
        binding: &FshResolvedPattern<'a>,
        _init_state: &mut State<'a, FshQueryContext>,
        _context: &'a FshExecContext,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<bool> {
        let Some(binding) = binding.get_last_binding() else {
            return Ok(false);
        };
        let Some(node) = binding.singleton() else {
            return Ok(false);
        };
        if node.kind() != self.kind {
            return Ok(false);
        }
        Ok(true)
    }
}

impl PatternName for FshNodePattern {
    fn name(&self) -> &'static str {
        "FshNode"
    }
}

impl AstLeafNodePattern<FshQueryContext> for FshLeafNodePattern {
    fn text(&self) -> Option<&str> {
        Some(&self.text)
    }
}

impl Matcher<FshQueryContext> for FshLeafNodePattern {
    fn execute<'a>(
        &'a self,
        binding: &FshResolvedPattern<'a>,
        _init_state: &mut State<'a, FshQueryContext>,
        _context: &'a FshExecContext,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<bool> {
        let Some(binding) = binding.get_last_binding() else {
            return Ok(false);
        };
        let Some(node) = binding.singleton() else {
            return Ok(false);
        };
        Ok(node.text_content() == self.text)
    }
}

impl PatternName for FshLeafNodePattern {
    fn name(&self) -> &'static str {
        "FshLeafNode"
    }
}

impl Matcher<FshQueryContext> for FshCodeSnippet {
    fn execute<'a>(
        &'a self,
        resolved: &FshResolvedPattern<'a>,
        state: &mut State<'a, FshQueryContext>,
        context: &'a FshExecContext,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<bool> {
        let Some(binding) = resolved.get_last_binding() else {
            return Ok(resolved.text(&state.files, context.language())?.trim() == self.source);
        };

        let Some(node) = binding.singleton() else {
            return Ok(false);
        };

        if let Some((_, pattern)) = self.patterns.iter().find(|(kind, _)| *kind == node.kind()) {
            pattern.execute(resolved, state, context, _logs)
        } else {
            Ok(false)
        }
    }
}

impl PatternName for FshCodeSnippet {
    fn name(&self) -> &'static str {
        "CodeSnippet"
    }
}

#[derive(Debug)]
pub struct FshFileInfo<'a> {
    pub path: PathBuf,
    pub tree: &'a FshGritTree,
}

pub struct FshExecContext<'a> {
    pub lang: FshTargetLanguage,
    pub name: Option<&'a str>,
    pub files: &'a FileOwners<FshGritTree>,
    pub functions: &'a [GritFunctionDefinition<FshQueryContext>],
    pub patterns: &'a [PatternDefinition<FshQueryContext>],
    pub predicates: &'a [PredicateDefinition<FshQueryContext>],
}

impl<'a> GritExecContext<'a, FshQueryContext> for FshExecContext<'a> {
    fn pattern_definitions(&self) -> &[PatternDefinition<FshQueryContext>] {
        self.patterns
    }

    fn predicate_definitions(&self) -> &[PredicateDefinition<FshQueryContext>] {
        self.predicates
    }

    fn function_definitions(&self) -> &[GritFunctionDefinition<FshQueryContext>] {
        self.functions
    }

    fn ignore_limit_pattern(&self) -> bool {
        false
    }

    fn call_built_in(
        &self,
        _call: &'a CallBuiltIn<FshQueryContext>,
        _context: &'a Self,
        _state: &mut State<'a, FshQueryContext>,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<FshResolvedPattern<'a>> {
        Ok(FshResolvedPattern::Constant(Constant::Boolean(true)))
    }

    fn call_callback<'b>(
        &self,
        _call: &'a grit_pattern_matcher::pattern::CallbackPattern,
        _context: &'a Self,
        _binding: &'b FshResolvedPattern<'a>,
        _state: &mut State<'a, FshQueryContext>,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<bool> {
        Ok(true)
    }

    fn files(&self) -> &FileOwners<FshGritTree> {
        self.files
    }

    fn language(&self) -> &FshTargetLanguage {
        &self.lang
    }

    fn exec_step(
        &'a self,
        _step: &'a Pattern<FshQueryContext>,
        _binding: &FshResolvedPattern,
        _state: &mut State<'a, FshQueryContext>,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<bool> {
        Ok(true)
    }

    fn name(&self) -> Option<&str> {
        self.name
    }

    fn load_file(
        &self,
        _file: &FshFile<'a>,
        _state: &mut State<'a, FshQueryContext>,
        _logs: &mut AnalysisLogs,
    ) -> GritResult<bool> {
        Ok(true)
    }
}
