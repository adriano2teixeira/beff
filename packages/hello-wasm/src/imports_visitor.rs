use bff_core::{ImportReference, TypeExport};
use std::collections::HashMap;
use swc_atoms::JsWord;
use swc_common::{FileName, SyntaxContext};
use swc_ecma_ast::Decl;
use swc_ecma_ast::ExportDecl;
use swc_ecma_ast::{
    ImportDecl, ImportNamedSpecifier, ImportSpecifier, TsInterfaceDecl, TsTypeAliasDecl,
};
use swc_ecma_visit::Visit;

pub struct ImportsVisitor {
    pub imports: HashMap<(JsWord, SyntaxContext), ImportReference>,
    pub type_exports: HashMap<JsWord, TypeExport>,
    pub current_file: FileName,
}

impl ImportsVisitor {
    pub fn from_file(current_file: FileName) -> ImportsVisitor {
        ImportsVisitor {
            imports: HashMap::new(),
            type_exports: HashMap::new(),
            current_file,
        }
    }
}

impl Visit for ImportsVisitor {
    fn visit_export_decl(&mut self, n: &ExportDecl) {
        match &n.decl {
            Decl::TsInterface(n) => {
                let TsInterfaceDecl { id, .. } = &**n;
                self.type_exports
                    .insert(id.sym.clone(), TypeExport::TsInterfaceDecl(*n.clone()));
            }
            Decl::TsTypeAlias(a) => {
                let TsTypeAliasDecl { id, type_ann, .. } = &**a;
                self.type_exports
                    .insert(id.sym.clone(), TypeExport::TsType(*type_ann.clone()));
            }
            Decl::Using(_)
            | Decl::Class(_)
            | Decl::Fn(_)
            | Decl::TsEnum(_)
            | Decl::TsModule(_)
            | Decl::Var(_) => {}
        }
    }

    fn visit_import_decl(&mut self, node: &ImportDecl) {
        let module_specifier = node.src.value.to_string();

        for x in &node.specifiers {
            match x {
                ImportSpecifier::Named(ImportNamedSpecifier { local, .. }) => {
                    let k = (local.sym.clone(), local.span.ctxt);

                    let v = crate::resolve_import(
                        &self.current_file.to_string().as_str(),
                        &module_specifier,
                    );

                    match v {
                        Some(v) => {
                            self.imports.insert(
                                k,
                                ImportReference {
                                    file_name: FileName::Real(v.into()),
                                },
                            );
                        }
                        None => {}
                    }
                }
                ImportSpecifier::Default(_) => todo!(),
                ImportSpecifier::Namespace(_) => todo!(),
            }
        }
    }
}