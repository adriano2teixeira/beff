use crate::api_extractor::FileManager;
use crate::diag::{
    span_to_loc, Diagnostic, DiagnosticInfoMessage, DiagnosticInformation, DiagnosticParentMessage,
};
use crate::open_api_ast::Json;
use crate::open_api_ast::{Definition, JsonSchema, Optionality};
use crate::type_resolve::{ResolvedLocalType, TypeResolver};
use crate::{BffFileName, ImportReference, TypeExport};
use std::collections::HashMap;
use std::rc::Rc;
use swc_atoms::JsWord;
use swc_common::Span;
use swc_ecma_ast::{
    BigInt, Expr, Ident, Str, TsArrayType, TsCallSignatureDecl, TsConditionalType,
    TsConstructSignatureDecl, TsConstructorType, TsEntityName, TsFnOrConstructorType, TsFnType,
    TsGetterSignature, TsImportType, TsIndexSignature, TsIndexedAccessType, TsInferType,
    TsInterfaceDecl, TsIntersectionType, TsKeywordType, TsKeywordTypeKind, TsLit, TsLitType,
    TsMappedType, TsMethodSignature, TsOptionalType, TsParenthesizedType, TsQualifiedName,
    TsRestType, TsSetterSignature, TsThisType, TsTplLitType, TsTupleType, TsType, TsTypeElement,
    TsTypeLit, TsTypeOperator, TsTypeParamInstantiation, TsTypePredicate, TsTypeQuery, TsTypeRef,
    TsUnionOrIntersectionType, TsUnionType,
};

pub struct TypeToSchema<'a, R: FileManager> {
    pub files: &'a mut R,
    pub current_file: BffFileName,
    pub components: HashMap<String, Option<Definition>>,
    pub ref_stack: Vec<DiagnosticInformation>,
}

fn extract_items_from_array(it: JsonSchema) -> JsonSchema {
    match it {
        JsonSchema::Array(items) => *items,
        _ => it,
    }
}

type Res<T> = Result<T, Diagnostic>;

impl<'a, R: FileManager> TypeToSchema<'a, R> {
    pub fn new(files: &'a mut R, current_file: BffFileName) -> TypeToSchema<'a, R> {
        TypeToSchema {
            files,
            current_file,
            components: HashMap::new(),
            ref_stack: vec![],
        }
    }
    fn ts_keyword_type_kind_to_json_schema(
        &mut self,
        kind: TsKeywordTypeKind,
        span: &Span,
    ) -> Res<JsonSchema> {
        match kind {
            TsKeywordTypeKind::TsUndefinedKeyword | TsKeywordTypeKind::TsNullKeyword => {
                Ok(JsonSchema::Null)
            }
            TsKeywordTypeKind::TsAnyKeyword
            | TsKeywordTypeKind::TsUnknownKeyword
            | TsKeywordTypeKind::TsObjectKeyword => Ok(JsonSchema::Any),
            TsKeywordTypeKind::TsBigIntKeyword
            | TsKeywordTypeKind::TsNeverKeyword
            | TsKeywordTypeKind::TsSymbolKeyword
            | TsKeywordTypeKind::TsIntrinsicKeyword
            | TsKeywordTypeKind::TsVoidKeyword => self.cannot_serialize_error(
                span,
                DiagnosticInfoMessage::KeywordNonSerializableToJsonSchema,
            ),
            TsKeywordTypeKind::TsNumberKeyword => Ok(JsonSchema::Number),
            TsKeywordTypeKind::TsBooleanKeyword => Ok(JsonSchema::Boolean),
            TsKeywordTypeKind::TsStringKeyword => Ok(JsonSchema::String),
        }
    }
    fn convert_ts_type_element(
        &mut self,
        prop: &TsTypeElement,
    ) -> Res<(String, Optionality<JsonSchema>)> {
        match prop {
            TsTypeElement::TsPropertySignature(prop) => {
                let key = match &*prop.key {
                    Expr::Ident(ident) => ident.sym.to_string(),
                    _ => {
                        return self.error(&prop.span, DiagnosticInfoMessage::PropKeyShouldBeIdent)
                    }
                };
                match &prop.type_ann.as_ref() {
                    Some(val) => {
                        let value = self.convert_ts_type(&val.type_ann)?;
                        let value = if prop.optional {
                            value.optional()
                        } else {
                            value.required()
                        };
                        Ok((key, value))
                    }
                    None => self.error(
                        &prop.span,
                        DiagnosticInfoMessage::PropShouldHaveTypeAnnotation,
                    ),
                }
            }
            TsTypeElement::TsGetterSignature(TsGetterSignature { span, .. })
            | TsTypeElement::TsSetterSignature(TsSetterSignature { span, .. })
            | TsTypeElement::TsMethodSignature(TsMethodSignature { span, .. })
            | TsTypeElement::TsIndexSignature(TsIndexSignature { span, .. })
            | TsTypeElement::TsCallSignatureDecl(TsCallSignatureDecl { span, .. })
            | TsTypeElement::TsConstructSignatureDecl(TsConstructSignatureDecl { span, .. }) => {
                self.cannot_serialize_error(
                    span,
                    DiagnosticInfoMessage::PropertyNonSerializableToJsonSchema,
                )
            }
        }
    }

    fn convert_ts_interface_decl(&mut self, typ: &TsInterfaceDecl) -> Res<JsonSchema> {
        if !typ.extends.is_empty() {
            return self.cannot_serialize_error(
                &typ.span,
                DiagnosticInfoMessage::TsInterfaceExtendsNotSupported,
            );
        }

        Ok(JsonSchema::Object {
            values: typ
                .body
                .body
                .iter()
                .map(|x| self.convert_ts_type_element(x))
                .collect::<Res<_>>()?,
        })
    }

    fn convert_type_export(
        &mut self,
        exported: &TypeExport,
        from_file: &BffFileName,
        span: &Span,
    ) -> Res<JsonSchema> {
        let store_current_file = self.current_file.clone();
        self.current_file = from_file.clone();
        let ty = match exported {
            TypeExport::TsType { ty: alias, .. } => self.convert_ts_type(alias)?,
            TypeExport::TsInterfaceDecl(int) => self.convert_ts_interface_decl(int)?,
            TypeExport::StarOfOtherFile(_) => {
                return self.error(span, DiagnosticInfoMessage::CannotUseStarAsType)
            }
            TypeExport::SomethingOfOtherFile(word, from_file) => {
                let file = self
                    .files
                    .get_or_fetch_file(from_file)
                    .and_then(|file| file.type_exports.get(word, self.files));
                match file {
                    Some(exported) => {
                        self.convert_type_export(exported.as_ref(), from_file, span)?
                    }
                    None => {
                        return self.error(
                            span,
                            DiagnosticInfoMessage::CannotResolveSomethingOfOtherFile,
                        )
                    }
                }
            }
        };
        self.current_file = store_current_file;
        Ok(ty)
    }

    fn get_type_ref_of_user_identifier(&mut self, i: &Ident) -> Res<JsonSchema> {
        match TypeResolver::new(self.files, &self.current_file).resolve_local_type(i)? {
            ResolvedLocalType::TsType(alias) => self.convert_ts_type(&alias),
            ResolvedLocalType::TsInterfaceDecl(int) => self.convert_ts_interface_decl(&int),
            ResolvedLocalType::NamedImport {
                exported,
                from_file,
            } => {
                return self.convert_type_export(exported.as_ref(), from_file.file_name(), &i.span)
            }
        }
    }

    fn insert_definition(&mut self, name: String, schema: JsonSchema) -> Res<JsonSchema> {
        self.components.insert(
            name.clone(),
            Some(Definition {
                name: name.clone(),
                schema,
            }),
        );
        Ok(JsonSchema::Ref(name))
    }

    fn get_string_with_format(
        &mut self,
        type_params: &Option<Box<TsTypeParamInstantiation>>,
        span: &Span,
    ) -> Res<JsonSchema> {
        let r = type_params.as_ref().and_then(|it| it.params.split_first());

        if let Some((head, rest)) = r {
            assert!(rest.is_empty());
            match &**head {
                TsType::TsLitType(TsLitType { lit, .. }) => match lit {
                    TsLit::Str(Str { value, .. }) => {
                        return Ok(JsonSchema::StringWithFormat(value.to_string()))
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        self.error(
            span,
            DiagnosticInfoMessage::InvalidUsageOfStringFormatTypeParameter,
        )
    }

    fn get_type_ref(
        &mut self,
        i: &Ident,
        type_params: &Option<Box<TsTypeParamInstantiation>>,
    ) -> Res<JsonSchema> {
        match i.sym.to_string().as_str() {
            "Array" => {
                let type_params = type_params.as_ref().and_then(|it| it.params.split_first());
                if let Some((ty, [])) = type_params {
                    let ty = self.convert_ts_type(ty)?;
                    return Ok(JsonSchema::Array(ty.into()));
                }
                return Ok(JsonSchema::Array(JsonSchema::Any.into()));
            }
            "StringFormat" => return self.get_string_with_format(type_params, &i.span),
            _ => {}
        }

        let found = self.components.get(&(i.sym.to_string()));
        if let Some(_found) = found {
            return Ok(JsonSchema::Ref(i.sym.to_string()));
        }
        self.components.insert(i.sym.to_string(), None);

        if type_params.is_some() {
            self.insert_definition(i.sym.to_string(), JsonSchema::Error)?;
            return self.cannot_serialize_error(
                &i.span,
                DiagnosticInfoMessage::TypeParameterApplicationNotSupported,
            );
        }

        let ty = self.get_type_ref_of_user_identifier(i);
        match ty {
            Ok(ty) => self.insert_definition(i.sym.to_string(), ty),
            Err(e) => {
                self.insert_definition(i.sym.to_string(), JsonSchema::Error)?;
                Err(e)
            }
        }
    }

    fn union(&mut self, types: &[Box<TsType>]) -> Res<JsonSchema> {
        Ok(JsonSchema::AnyOf(
            types
                .iter()
                .map(|it| self.convert_ts_type(it))
                .collect::<Res<_>>()?,
        ))
    }

    fn intersection(&mut self, types: &[Box<TsType>]) -> Res<JsonSchema> {
        Ok(JsonSchema::AllOf(
            types
                .iter()
                .map(|it| self.convert_ts_type(it))
                .collect::<Res<_>>()?,
        ))
    }

    fn cannot_serialize_error<T>(&mut self, span: &Span, msg: DiagnosticInfoMessage) -> Res<T> {
        let cause = self.create_error(span, msg);

        match self.ref_stack.split_first() {
            Some((head, tail)) => {
                let mut related_information = tail.to_vec();
                related_information.push(cause);
                Err(Diagnostic {
                    cause: head.clone(),
                    related_information: Some(related_information),
                    parent_big_message: Some(DiagnosticParentMessage::CannotConvertToSchema),
                })
            }
            None => Err(Diagnostic {
                parent_big_message: Some(DiagnosticParentMessage::CannotConvertToSchema),
                cause,
                related_information: None,
            }),
        }
    }

    fn create_error(&mut self, span: &Span, msg: DiagnosticInfoMessage) -> DiagnosticInformation {
        let file = self.files.get_or_fetch_file(&self.current_file);
        match file {
            Some(file) => {
                let (loc_lo, loc_hi) =
                    span_to_loc(span, &file.module.source_map, file.module.fm.end_pos);

                DiagnosticInformation::KnownFile {
                    message: msg,
                    file_name: self.current_file.clone(),
                    loc_hi,
                    loc_lo,
                }
            }
            None => DiagnosticInformation::UnfoundFile {
                message: msg,
                current_file: self.current_file.clone(),
            },
        }
    }
    fn error<T>(&mut self, span: &Span, msg: DiagnosticInfoMessage) -> Res<T> {
        let err = self.create_error(span, msg);
        Err(err.to_diag(None))
    }
    fn get_identifier_diag_info(&mut self, i: &Ident) -> Option<DiagnosticInformation> {
        self.files
            .get_or_fetch_file(&self.current_file)
            .map(|file| {
                let (loc_lo, loc_hi) =
                    span_to_loc(&i.span, &file.module.source_map, file.module.fm.end_pos);

                DiagnosticInformation::KnownFile {
                    file_name: self.current_file.clone(),
                    loc_hi,
                    loc_lo,
                    message: DiagnosticInfoMessage::ThisRefersToSomethingThatCannotBeSerialized(
                        i.sym.to_string(),
                    ),
                }
            })
    }

    fn get_qualified_type_from_file(
        &mut self,
        from_file: &Rc<ImportReference>,
        right: &JsWord,
        span: &Span,
    ) -> Res<(Rc<TypeExport>, Rc<ImportReference>, String)> {
        let exported = self
            .files
            .get_or_fetch_file(from_file.file_name())
            .and_then(|module| {
                module
                    .type_exports
                    .get(right, self.files)
            });
        match exported {
            Some(exported) => {
                let name = match &*exported {
                    TypeExport::TsType { name, .. } => name.to_string(),
                    TypeExport::TsInterfaceDecl(it) => it.id.sym.to_string(),
                    TypeExport::StarOfOtherFile(_) => right.to_string(),
                    TypeExport::SomethingOfOtherFile(that, _) => that.to_string(),
                };
                Ok((exported, from_file.clone(), name))
            }
            None => self.error(span, DiagnosticInfoMessage::CannotGetQualifiedTypeFromFile),
        }
    }

    fn recursively_get_qualified_type_export(
        &mut self,
        exported: Rc<TypeExport>,
        right: &Ident,
    ) -> Res<(Rc<TypeExport>, Rc<ImportReference>, String)> {
        match &*exported {
            TypeExport::TsType { .. } => self.error(
                &right.span,
                DiagnosticInfoMessage::CannotUseTsTypeAsQualified,
            ),
            TypeExport::TsInterfaceDecl(_) => self.error(
                &right.span,
                DiagnosticInfoMessage::CannotUseTsInterfaceAsQualified,
            ),
            TypeExport::StarOfOtherFile(other_file) => {
                self.get_qualified_type_from_file(other_file, &right.sym, &right.span)
            }
            TypeExport::SomethingOfOtherFile(word, from_file) => {
                let exported = self.files.get_or_fetch_file(from_file).and_then(|module| {
                    module
                        .type_exports
                        .get(word, self.files)
                });

                match exported {
                    Some(exported) => self.recursively_get_qualified_type_export(exported, right),
                    None => self.error(
                        &right.span,
                        DiagnosticInfoMessage::CannotGetQualifiedTypeFromFile,
                    ),
                }
            }
        }
    }
    fn __convert_ts_type_qual_inner(
        &mut self,
        q: &TsQualifiedName,
    ) -> Res<(Rc<TypeExport>, Rc<ImportReference>, String)> {
        match &q.left {
            TsEntityName::TsQualifiedName(q2) => {
                let (exported, _from_file, _name) = self.__convert_ts_type_qual_inner(q2)?;
                self.recursively_get_qualified_type_export(exported, &q.right)
            }
            TsEntityName::Ident(i) => {
                let ns =
                    TypeResolver::new(self.files, &self.current_file).resolve_namespace_type(i)?;
                self.get_qualified_type_from_file(&ns.from_file, &q.right.sym, &q.right.span)
            }
        }
    }

    fn __convert_ts_type_qual_caching(&mut self, q: &TsQualifiedName) -> Res<JsonSchema> {
        let found = self.components.get(&(q.right.sym.to_string()));
        if let Some(_found) = found {
            return Ok(JsonSchema::Ref(q.right.sym.to_string()));
        }

        let (exported, from_file, name) = self.__convert_ts_type_qual_inner(q)?;
        let ty =
            self.convert_type_export(exported.as_ref(), from_file.file_name(), &q.right.span)?;
        self.insert_definition(name, ty)
    }
    fn convert_ts_type_qual(&mut self, q: &TsQualifiedName) -> Res<JsonSchema> {
        let current_ref = self.get_identifier_diag_info(&q.right);
        let did_push = current_ref.is_some();
        if let Some(current_ref) = current_ref {
            self.ref_stack.push(current_ref);
        }
        let v = self.__convert_ts_type_qual_caching(q);
        if did_push {
            self.ref_stack.pop();
        }
        v
    }
    fn convert_ts_type_ident(
        &mut self,
        i: &Ident,
        type_params: &Option<Box<TsTypeParamInstantiation>>,
    ) -> Res<JsonSchema> {
        let current_ref = self.get_identifier_diag_info(i);
        let did_push = current_ref.is_some();
        if let Some(current_ref) = current_ref {
            self.ref_stack.push(current_ref);
        }
        let v = self.get_type_ref(i, type_params);
        if did_push {
            self.ref_stack.pop();
        }
        v
    }
    pub fn convert_ts_type(&mut self, typ: &TsType) -> Res<JsonSchema> {
        match typ {
            TsType::TsKeywordType(TsKeywordType { kind, span, .. }) => {
                self.ts_keyword_type_kind_to_json_schema(*kind, span)
            }
            TsType::TsTypeRef(TsTypeRef {
                type_name,
                type_params,
                ..
            }) => match &type_name {
                TsEntityName::Ident(i) => self.convert_ts_type_ident(i, type_params),
                TsEntityName::TsQualifiedName(q) => {
                    assert!(type_params.is_none());
                    self.convert_ts_type_qual(q)
                }
            },
            TsType::TsTypeLit(TsTypeLit { members, .. }) => Ok(JsonSchema::Object {
                values: members
                    .iter()
                    .map(|prop| self.convert_ts_type_element(prop))
                    .collect::<Res<_>>()?,
            }),
            TsType::TsArrayType(TsArrayType { elem_type, .. }) => {
                Ok(JsonSchema::Array(self.convert_ts_type(elem_type)?.into()))
            }
            TsType::TsUnionOrIntersectionType(it) => match &it {
                TsUnionOrIntersectionType::TsUnionType(TsUnionType { types, .. }) => {
                    self.union(types)
                }
                TsUnionOrIntersectionType::TsIntersectionType(TsIntersectionType {
                    types, ..
                }) => self.intersection(types),
            },
            TsType::TsTupleType(TsTupleType { elem_types, .. }) => {
                let mut prefix_items = vec![];
                let mut items = None;
                for it in elem_types {
                    if let TsType::TsRestType(TsRestType { type_ann, .. }) = &*it.ty {
                        if items.is_some() {
                            return self.cannot_serialize_error(
                                &it.span,
                                DiagnosticInfoMessage::DuplicatedRestNonSerializableToJsonSchema,
                            );
                        }
                        let ann = extract_items_from_array(self.convert_ts_type(type_ann)?);
                        items = Some(ann.into());
                    } else {
                        let ty_schema = self.convert_ts_type(&it.ty)?;
                        prefix_items.push(ty_schema);
                    }
                }
                Ok(JsonSchema::Tuple {
                    prefix_items,
                    items,
                })
            }
            TsType::TsLitType(TsLitType { lit, .. }) => match lit {
                TsLit::Number(n) => Ok(JsonSchema::Const(Json::Number(n.value))),
                TsLit::Str(s) => Ok(JsonSchema::Const(Json::String(s.value.to_string().clone()))),
                TsLit::Bool(b) => Ok(JsonSchema::Const(Json::Bool(b.value))),
                TsLit::BigInt(BigInt { span, .. }) => self.cannot_serialize_error(
                    span,
                    DiagnosticInfoMessage::BigIntNonSerializableToJsonSchema,
                ),
                TsLit::Tpl(TsTplLitType {
                    span,
                    types,
                    quasis,
                }) => {
                    if !types.is_empty() || quasis.len() != 1 {
                        return self.cannot_serialize_error(
                            span,
                            DiagnosticInfoMessage::TemplateNonSerializableToJsonSchema,
                        );
                    }

                    Ok(JsonSchema::Const(Json::String(
                        quasis
                            .iter()
                            .map(|it| it.raw.to_string())
                            .collect::<String>(),
                    )))
                }
            },
            TsType::TsParenthesizedType(TsParenthesizedType { type_ann, .. }) => {
                self.convert_ts_type(type_ann)
            }
            TsType::TsOptionalType(TsOptionalType { span, .. }) => {
                self.error(span, DiagnosticInfoMessage::OptionalTypeIsNotSupported)
            }
            TsType::TsRestType(_) => unreachable!("should have been handled by parent node"),
            TsType::TsThisType(TsThisType { span, .. })
            | TsType::TsFnOrConstructorType(
                TsFnOrConstructorType::TsConstructorType(TsConstructorType { span, .. })
                | TsFnOrConstructorType::TsFnType(TsFnType { span, .. }),
            )
            | TsType::TsConditionalType(TsConditionalType { span, .. })
            | TsType::TsInferType(TsInferType { span, .. })
            | TsType::TsTypeOperator(TsTypeOperator { span, .. })
            | TsType::TsMappedType(TsMappedType { span, .. })
            | TsType::TsTypePredicate(TsTypePredicate { span, .. })
            | TsType::TsImportType(TsImportType { span, .. })
            | TsType::TsTypeQuery(TsTypeQuery { span, .. }) => self.cannot_serialize_error(
                span,
                DiagnosticInfoMessage::TypeConstructNonSerializableToJsonSchema,
            ),
            TsType::TsIndexedAccessType(TsIndexedAccessType { span, .. }) => self.error(
                span,
                DiagnosticInfoMessage::CannotUnderstandTsIndexedAccessType,
            ),
        }
    }
}