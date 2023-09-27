pub mod api_extractor;
pub mod ast;
pub mod diag;
pub mod emit;
pub mod import_resolver;
pub mod open_api_ast;
pub mod parse;
pub mod parser_extractor;
pub mod print;
pub mod schema_changes;
pub mod subtyping;
pub mod sym_reference;
pub mod type_to_schema;
pub mod wasm_diag;

use api_extractor::extract_schema;
use api_extractor::RouterExtractResult;
use core::fmt;
use diag::Diagnostic;
use open_api_ast::Validator;
use parser_extractor::extract_parser;
use parser_extractor::ParserExtractResult;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use swc_atoms::JsWord;
use swc_common::SourceFile;
use swc_common::SourceMap;
use swc_common::Span;
use swc_common::SyntaxContext;
use swc_ecma_ast::Decl;
use swc_ecma_ast::Expr;
use swc_ecma_ast::ModuleItem;
use swc_ecma_ast::Pat;
use swc_ecma_ast::Stmt;
use swc_ecma_ast::TsTypeParamDecl;
use swc_ecma_ast::{Module, TsType};
use swc_ecma_ast::{TsInterfaceDecl, TsTypeAliasDecl};
use swc_ecma_visit::Visit;
use swc_node_comments::SwcComments;

#[derive(Debug, Clone)]
pub enum SymbolExport {
    TsTypeTemplate {
        params: Rc<TsTypeParamDecl>,
        ty: Rc<TsType>,
        name: JsWord,
    },
    TsType {
        ty: Rc<TsType>,
        name: JsWord,
    },
    TsInterfaceDecl(Rc<TsInterfaceDecl>),
    ValueExpr {
        expr: Rc<Expr>,
        name: JsWord,
    },
    StarOfOtherFile(Rc<ImportReference>),
    SomethingOfOtherFile(JsWord, BffFileName),
}

pub struct BffModuleData {
    pub bff_fname: BffFileName,
    pub fm: Arc<SourceFile>,
    pub source_map: Arc<SourceMap>,
    pub module: Module,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BffFileName(Rc<String>);

impl fmt::Display for BffFileName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl BffFileName {
    pub fn new(s: String) -> BffFileName {
        BffFileName(Rc::new(s))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone)]
pub enum ImportReference {
    Named {
        orig: Rc<JsWord>,
        file_name: BffFileName,
    },
    Star {
        file_name: BffFileName,
    },
    Default {
        file_name: BffFileName,
    },
}

impl ImportReference {
    pub fn file_name(&self) -> &BffFileName {
        match self {
            ImportReference::Named { file_name, .. } => file_name,
            ImportReference::Star { file_name, .. } => file_name,
            ImportReference::Default { file_name, .. } => file_name,
        }
    }
}
#[derive(Debug, Clone)]
pub struct SymbolsExportsModule {
    named: HashMap<JsWord, Rc<SymbolExport>>,
    extends: Vec<BffFileName>,
}
impl Default for SymbolsExportsModule {
    fn default() -> Self {
        Self::new()
    }
}
impl SymbolsExportsModule {
    pub fn new() -> SymbolsExportsModule {
        SymbolsExportsModule {
            named: HashMap::new(),
            extends: Vec::new(),
        }
    }

    pub fn insert(&mut self, name: JsWord, export: Rc<SymbolExport>) {
        self.named.insert(name, export);
    }

    pub fn get<R: FileManager>(&self, name: &JsWord, files: &mut R) -> Option<Rc<SymbolExport>> {
        self.named.get(name).cloned().or_else(|| {
            for it in &self.extends {
                let file = files.get_or_fetch_file(it)?;
                let res = file.symbol_exports.get(name, files);
                if let Some(it) = res {
                    return Some(it.clone());
                }
            }
            None
        })
    }

    pub fn extend(&mut self, other: BffFileName) {
        self.extends.push(other);
    }
}

pub struct SymbolExportDefault {
    pub symbol_export: Rc<Expr>,
    pub span: Span,
    pub file_name: BffFileName,
}

pub struct ParsedModule {
    pub locals: ParsedModuleLocals,
    pub module: BffModuleData,
    pub imports: HashMap<(JsWord, SyntaxContext), Rc<ImportReference>>,
    pub comments: SwcComments,
    pub symbol_exports: SymbolsExportsModule,
    pub export_default: Option<Rc<SymbolExportDefault>>,
}

#[derive(Debug)]
pub struct ParsedModuleLocals {
    pub type_aliases: HashMap<(JsWord, SyntaxContext), Rc<TsType>>,
    pub type_templates: HashMap<(JsWord, SyntaxContext), (Rc<TsTypeParamDecl>, Rc<TsType>)>,
    pub interfaces: HashMap<(JsWord, SyntaxContext), Rc<TsInterfaceDecl>>,

    pub exprs: HashMap<(JsWord, SyntaxContext), Rc<Expr>>,
}
impl ParsedModuleLocals {
    pub fn new() -> ParsedModuleLocals {
        ParsedModuleLocals {
            type_aliases: HashMap::new(),
            interfaces: HashMap::new(),
            exprs: HashMap::new(),
            type_templates: HashMap::new(),
        }
    }
}

impl Default for ParsedModuleLocals {
    fn default() -> Self {
        Self::new()
    }
}
pub struct ParserOfModuleLocals {
    content: ParsedModuleLocals,
}
impl Default for ParserOfModuleLocals {
    fn default() -> Self {
        Self::new()
    }
}
impl ParserOfModuleLocals {
    pub fn new() -> ParserOfModuleLocals {
        ParserOfModuleLocals {
            content: ParsedModuleLocals::new(),
        }
    }

    pub fn visit_module_item_list(&mut self, it: &[ModuleItem]) {
        for it in it {
            match it {
                ModuleItem::Stmt(Stmt::Decl(decl)) => {
                    // add expr to self.content

                    if let Decl::Var(decls) = decl {
                        for it in &decls.decls {
                            if let Some(expr) = &it.init {
                                if let Pat::Ident(id) = &it.name {
                                    self.content.exprs.insert(
                                        (id.sym.clone(), id.span.ctxt),
                                        Rc::new(*expr.clone()),
                                    );
                                }
                            }
                        }
                    }
                }
                ModuleItem::ModuleDecl(_) => {}
                ModuleItem::Stmt(_) => {}
            }
        }
    }
}

impl Visit for ParserOfModuleLocals {
    fn visit_ts_type_alias_decl(&mut self, n: &TsTypeAliasDecl) {
        let TsTypeAliasDecl {
            id,
            type_ann,
            type_params,
            ..
        } = n;

        match type_params {
            Some(p) => {
                self.content.type_templates.insert(
                    (id.sym.clone(), id.span.ctxt),
                    (Rc::new(*p.clone()), Rc::new(*type_ann.clone())),
                );
            }
            None => {
                self.content
                    .type_aliases
                    .insert((id.sym.clone(), id.span.ctxt), Rc::new(*type_ann.clone()));
            }
        }
    }
    fn visit_ts_interface_decl(&mut self, n: &TsInterfaceDecl) {
        let TsInterfaceDecl { id, .. } = n;
        self.content
            .interfaces
            .insert((id.sym.clone(), id.span.ctxt), Rc::new(n.clone()));
    }
}

pub struct UnresolvedExport {
    pub name: JsWord,
    pub span: SyntaxContext,
    pub renamed: JsWord,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BeffUserSettings {
    pub custom_formats: BTreeSet<String>,
}

pub struct EntryPoints {
    pub router_entry_point: Option<BffFileName>,
    pub parser_entry_point: Option<BffFileName>,
    pub settings: BeffUserSettings,
}
pub trait FileManager {
    fn get_or_fetch_file(&mut self, name: &BffFileName) -> Option<Rc<ParsedModule>>;
    fn get_existing_file(&self, name: &BffFileName) -> Option<Rc<ParsedModule>>;
}

pub struct ExtractResult {
    pub router: Option<RouterExtractResult>,
    pub parser: Option<ParserExtractResult>,
}

impl ExtractResult {
    pub fn is_empty(&self) -> bool {
        self.router.is_none() && self.parser.is_none()
    }
    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.router
            .as_ref()
            .map(|it| it.errors.iter().collect())
            .unwrap_or(vec![])
            .into_iter()
            .chain(
                self.parser
                    .as_ref()
                    .map(|it| it.errors.iter().collect())
                    .unwrap_or(vec![]),
            )
            .collect()
    }
    pub fn validators(&self) -> Vec<&Validator> {
        self.router
            .as_ref()
            .map(|it| it.validators.iter().collect())
            .unwrap_or(vec![])
            .into_iter()
            .chain(
                self.parser
                    .as_ref()
                    .map(|it| it.validators.iter().collect())
                    .unwrap_or(vec![]),
            )
            .collect()
    }
}
pub fn extract<R: FileManager>(files: &mut R, entry_points: EntryPoints) -> ExtractResult {
    let mut router = None;
    let mut parser = None;

    if let Some(entry) = entry_points.router_entry_point {
        router = Some(extract_schema(files, entry, &entry_points.settings));
    }
    if let Some(entry) = entry_points.parser_entry_point {
        parser = Some(extract_parser(files, entry, &entry_points.settings));
    }

    ExtractResult { router, parser }
}
