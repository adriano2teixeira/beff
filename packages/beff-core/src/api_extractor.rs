use crate::diag::{
    span_to_loc, Diagnostic, DiagnosticInfoMessage, DiagnosticInformation, DiagnosticParentMessage,
};
use crate::open_api_ast::{
    self, Definition, Info, JsonRequestBody, JsonSchema, OpenApi, OperationObject, ParameterIn,
    ParameterObject,
};
use crate::type_to_schema::TypeToSchema;
use crate::{BffFileName, ParsedModule};
use anyhow::anyhow;
use anyhow::Result;
use core::fmt;
use jsdoc::ast::{SummaryTag, Tag, UnknownTag, VersionTag};
use jsdoc::Input;
use std::collections::HashSet;
use std::rc::Rc;
use swc_common::comments::{Comment, CommentKind, Comments};
use swc_common::{BytePos, Span, DUMMY_SP};
use swc_ecma_ast::{
    ArrayPat, ArrowExpr, AssignPat, AssignProp, BigInt, BindingIdent, CallExpr, Callee,
    ComputedPropName, ExportDefaultExpr, Expr, FnExpr, Function, GetterProp, Ident, Invalid,
    KeyValueProp, Lit, MethodProp, Number, ObjectPat, Pat, Prop, PropName, PropOrSpread, RestPat,
    SetterProp, SpreadElement, Str, Tpl, TsCallSignatureDecl, TsConstructSignatureDecl,
    TsEntityName, TsGetterSignature, TsIndexSignature, TsKeywordType, TsKeywordTypeKind,
    TsMethodSignature, TsPropertySignature, TsSetterSignature, TsType, TsTypeAnn, TsTypeElement,
    TsTypeLit, TsTypeParamDecl, TsTypeParamInstantiation, TsTypeRef,
};
use swc_ecma_visit::Visit;

fn maybe_extract_promise(typ: &TsType) -> &TsType {
    if let TsType::TsTypeRef(refs) = typ {
        if let TsEntityName::Ident(i) = &refs.type_name {
            // if name is promise
            if i.sym == *"Promise" {
                match refs.type_params.as_ref() {
                    Some(inst) => {
                        let ts_type = inst.params.get(0);
                        match ts_type {
                            Some(ts_type) => return ts_type,
                            None => {}
                        }
                    }
                    None => {}
                }
            }
        }
    }
    typ
}

#[derive(Debug, Clone)]
pub enum HeaderOrCookie {
    Header,
    Cookie,
}
impl fmt::Display for HeaderOrCookie {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HeaderOrCookie::Header => write!(f, "header"),
            HeaderOrCookie::Cookie => write!(f, "cookie"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum HandlerParameter {
    PathOrQueryOrBody {
        schema: JsonSchema,
        required: bool,
        description: Option<String>,
        span: Span,
    },
    HeaderOrCookie {
        span: Span,
        kind: HeaderOrCookie,
        schema: JsonSchema,
        required: bool,
        description: Option<String>,
    },
    Context(Span),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MethodKind {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Options,
    Use,
}
impl fmt::Display for MethodKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MethodKind::Get => write!(f, "get"),
            MethodKind::Post => write!(f, "post"),
            MethodKind::Put => write!(f, "put"),
            MethodKind::Delete => write!(f, "delete"),
            MethodKind::Patch => write!(f, "patch"),
            MethodKind::Options => write!(f, "options"),
            MethodKind::Use => write!(f, "use"),
        }
    }
}

impl MethodKind {
    pub fn text_len(&self) -> usize {
        self.to_string().len()
    }
}

#[derive(Debug, Clone)]
pub struct ParsedPattern {
    pub raw_span: Span,
    pub open_api_pattern: String,
    pub path_params: Vec<String>,
}

fn parse_pattern_params(pattern: &str) -> Vec<String> {
    // parse open api parameters from pattern
    let mut params = vec![];
    let chars = pattern.chars();
    let mut current = String::new();
    for c in chars {
        if c == '{' {
            current = String::new();
        } else if c == '}' {
            params.push(current.clone());
        } else {
            current.push(c);
        }
    }
    params
}

#[derive(Debug, Clone)]
pub struct FnHandler {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<(String, HandlerParameter)>,
    pub return_type: JsonSchema,
    pub method_kind: MethodKind,
    pub span: Span,
}

pub trait FileManager {
    fn get_or_fetch_file(&mut self, name: &BffFileName) -> Option<Rc<ParsedModule>>;
    fn get_existing_file(&self, name: &BffFileName) -> Option<Rc<ParsedModule>>;
}

pub struct BuiltDecoder {
    pub exported_name: String,
    pub schema: JsonSchema,
}

pub struct ExtractExportDefaultVisitor<'a, R: FileManager> {
    files: &'a mut R,
    current_file: BffFileName,
    handlers: Vec<PathHandlerMap>,
    components: Vec<Definition>,
    public_definitions: HashSet<String>,
    found_default_export: bool,
    errors: Vec<Diagnostic>,
    info: open_api_ast::Info,
    built_decoders: Option<Vec<BuiltDecoder>>,
}
impl<'a, R: FileManager> ExtractExportDefaultVisitor<'a, R> {
    fn new(files: &'a mut R, current_file: BffFileName) -> ExtractExportDefaultVisitor<'a, R> {
        ExtractExportDefaultVisitor {
            files,
            current_file,
            handlers: vec![],
            components: vec![],
            found_default_export: false,
            errors: vec![],
            info: open_api_ast::Info {
                title: None,
                description: None,
                version: None,
            },
            built_decoders: None,
            public_definitions: HashSet::new(),
        }
    }
}

fn trim_description_comments(it: String) -> String {
    // remove all spaces and * from end of string
    let mut v: Vec<char> = it.chars().collect();

    if v.is_empty() {
        return it;
    }

    while v
        .last()
        .expect("we just checked that it is not empty")
        .is_ascii_whitespace()
        || v.last().expect("we just checked that it is not empty") == &'*'
    {
        v.pop();
    }

    v.into_iter().collect()
}

struct EndpointComments {
    pub summary: Option<String>,
    pub description: Option<String>,
}
fn get_prop_name_span(key: &PropName) -> Span {
    let span = match key {
        PropName::Computed(ComputedPropName { span, .. })
        | PropName::Ident(Ident { span, .. })
        | PropName::Str(Str { span, .. })
        | PropName::Num(Number { span, .. })
        | PropName::BigInt(BigInt { span, .. }) => span,
    };
    *span
}
impl<'a, R: FileManager> ExtractExportDefaultVisitor<'a, R> {
    fn build_error(&self, span: &Span, msg: DiagnosticInfoMessage) -> DiagnosticInformation {
        let file = self.files.get_existing_file(&self.current_file);
        match file {
            Some(file) => {
                let (loc_lo, loc_hi) =
                    span_to_loc(span, &file.module.source_map, file.module.fm.end_pos);

                DiagnosticInformation::KnownFile {
                    message: msg,
                    file_name: self.current_file.clone(),
                    loc_lo,
                    loc_hi,
                }
            }
            None => DiagnosticInformation::UnfoundFile {
                message: msg,
                current_file: self.current_file.clone(),
            },
        }
    }
    fn push_error(&mut self, span: &Span, msg: DiagnosticInfoMessage) {
        self.errors.push(self.build_error(span, msg).to_diag(None));
    }

    fn error<T>(&mut self, span: &Span, msg: DiagnosticInfoMessage) -> Result<T> {
        let e = anyhow!("{:?}", &msg);
        self.errors.push(self.build_error(span, msg).to_diag(None));
        Err(e)
    }

    #[allow(clippy::to_string_in_format_args)]
    fn parse_endpoint_comments(&mut self, acc: &mut EndpointComments, comments: Vec<Comment>) {
        for c in comments {
            if c.kind == CommentKind::Block {
                let s = c.text;
                let parsed =
                    jsdoc::parse(Input::new(BytePos(0), BytePos(s.as_bytes().len() as _), &s));
                match parsed {
                    Ok((rest, parsed)) => {
                        acc.description = Some(trim_description_comments(
                            parsed.description.value.to_string(),
                        ));
                        if rest.len() > 0 {
                            acc.description = Some(format!(
                                "{}\n{}",
                                acc.description.as_ref().unwrap_or(&String::new()),
                                rest.to_string()
                            ));
                        }
                        for tag_item in parsed.tags {
                            match tag_item.tag {
                                Tag::Summary(SummaryTag { text, .. }) => {
                                    acc.summary = Some(text.value.to_string());
                                }
                                _tag => self.push_error(
                                    &tag_item.span,
                                    DiagnosticInfoMessage::UnknownJsDocTagOnEndpoint(
                                        tag_item.tag_name.value.to_string(),
                                    ),
                                ),
                            }
                        }
                    }
                    Err(_) => {
                        self.push_error(&c.span, DiagnosticInfoMessage::CannotParseJsDocEndpoint)
                    }
                }
            }
        }
    }
    fn parse_description_comment(&mut self, comments: Vec<Comment>, span: &Span) -> Option<String> {
        if comments.len() != 1 {
            self.push_error(span, DiagnosticInfoMessage::TooManyCommentsJsDoc);

            return None;
        }
        let first = comments
            .into_iter()
            .next()
            .expect("we just checked the length");
        if first.kind == CommentKind::Block {
            let s = first.text;
            let parsed = jsdoc::parse(Input::new(BytePos(0), BytePos(s.as_bytes().len() as _), &s));
            match parsed {
                Ok((rest, parsed)) => {
                    if !rest.is_empty() {
                        self.push_error(
                            &first.span,
                            DiagnosticInfoMessage::JsDocDescriptionRestIsNotEmpty,
                        );
                    }
                    if !parsed.tags.is_empty() {
                        self.push_error(
                            &first.span,
                            DiagnosticInfoMessage::JsDocsParameterDescriptionHasTags,
                        );
                    }
                    return Some(trim_description_comments(
                        parsed.description.value.to_string(),
                    ));
                }
                Err(_) => self.push_error(
                    &first.span,
                    DiagnosticInfoMessage::JsDocsDescriptionCouldNotBeParsed,
                ),
            }
        }
        None
    }

    fn parse_raw_pattern_str(&mut self, key: &str, span: &Span) -> Result<ParsedPattern> {
        let path_params = parse_pattern_params(key);
        Ok(ParsedPattern {
            raw_span: *span,
            open_api_pattern: key.to_string(),
            path_params,
        })
    }
    fn parse_key(&mut self, key: &PropName) -> Result<ParsedPattern> {
        match key {
            PropName::Computed(ComputedPropName { expr, span, .. }) => match &**expr {
                Expr::Lit(Lit::Str(Str { span, value, .. })) => {
                    self.parse_raw_pattern_str(value.as_ref(), span)
                }
                Expr::Tpl(Tpl {
                    span,
                    exprs,
                    quasis,
                }) => {
                    if !exprs.is_empty() {
                        return self
                            .error(span, DiagnosticInfoMessage::TemplateMustBeOfSingleString);
                    }
                    if quasis.len() != 1 {
                        return self
                            .error(span, DiagnosticInfoMessage::TemplateMustBeOfSingleString);
                    }
                    let first_quasis = quasis.first().expect("we just checked the length");
                    self.parse_raw_pattern_str(first_quasis.raw.as_ref(), span)
                }
                _ => self.error(
                    span,
                    DiagnosticInfoMessage::MustBeComputedKeyWithMethodAndPatternMustBeString,
                ),
            },
            PropName::Ident(Ident { span, .. })
            | PropName::Str(Str { span, .. })
            | PropName::Num(Number { span, .. })
            | PropName::BigInt(BigInt { span, .. }) => {
                self.error(span, DiagnosticInfoMessage::PatternMustBeComputedKey)
            }
        }
    }

    fn extend_components(&mut self, defs: Vec<Definition>, span: &Span) {
        for d in defs {
            let found = self.components.iter_mut().find(|x| x.name == d.name);
            if let Some(found) = found {
                if found.schema != d.schema {
                    self.push_error(
                        span,
                        DiagnosticInfoMessage::TwoDifferentTypesWithTheSameName,
                    );
                }
            } else {
                self.components.push(d);
            }
        }
    }

    fn convert_to_json_schema(&mut self, ty: &TsType, span: &Span, is_public: bool) -> JsonSchema {
        let mut to_schema = TypeToSchema::new(self.files, self.current_file.clone());
        let res = to_schema.convert_ts_type(ty);
        match res {
            Ok(res) => {
                let mut kvs = vec![];
                for (k, v) in to_schema.components {
                    // We store type in an Option to support self-recursion.
                    // When we encounter the type while transforming it we return string with the type name.
                    // And we need the option to allow a type to refer to itself before it has been resolved.
                    match v {
                        Some(s) => {
                            if is_public {
                                self.public_definitions.insert(k.clone());
                            }

                            kvs.push((k, s))
                        }
                        None => self.push_error(
                            span,
                            DiagnosticInfoMessage::CannotResolveTypeReferenceOnExtracting(k),
                        ),
                    }
                }

                kvs.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));
                let ext: Vec<Definition> = kvs.into_iter().map(|(_k, v)| v).collect();
                self.extend_components(ext, span);

                res
            }
            Err(diag) => {
                self.errors.push(diag);
                JsonSchema::Error
            }
        }
    }
    fn parse_lib_param(
        &mut self,
        lib_ty_name: &Ident,
        params: &Option<Box<TsTypeParamInstantiation>>,
        required: bool,
        description: Option<String>,
    ) -> Result<HandlerParameter> {
        let name = lib_ty_name.sym.to_string();
        let name = name.as_str();
        match name {
            "Header" | "Cookie" => {
                let params = &params.as_ref().and_then(|it| it.params.split_first());
                match params {
                    Some((ty, [])) => Ok(HandlerParameter::HeaderOrCookie {
                        kind: if name == "Header" {
                            HeaderOrCookie::Header
                        } else {
                            HeaderOrCookie::Cookie
                        },
                        span: lib_ty_name.span,
                        schema: self.convert_to_json_schema(ty, &lib_ty_name.span, true),
                        required,
                        description,
                    }),
                    _ => {
                        self.error(
                            &lib_ty_name.span,
                            DiagnosticInfoMessage::TooManyParamsOnLibType,
                        )
                    }
                }
            }
            _ => unreachable!("not in lib: {} - should check before calling", name),
        }
    }

    fn parse_type_ref_parameter(
        &mut self,
        tref: &TsTypeRef,
        ty: &TsType,
        required: bool,
        description: Option<String>,
    ) -> Result<HandlerParameter> {
        if let TsEntityName::Ident(i) = &tref.type_name {
            let name = i.sym.to_string();
            if name == "Ctx" || name == "Context" {
                return Ok(HandlerParameter::Context(i.span));
            }
            if name == "Header" || name == "Cookie" {
                return self.parse_lib_param(i, &tref.type_params, required, description);
            }
        }
        Ok(HandlerParameter::PathOrQueryOrBody {
            schema: self.convert_to_json_schema(ty, &tref.span, true),
            required,
            description,
            span: tref.span,
        })
    }
    fn parse_parameter_type(
        &mut self,
        ty: &TsType,
        required: bool,
        description: Option<String>,
        span: &Span,
    ) -> Result<HandlerParameter> {
        match ty {
            TsType::TsTypeRef(tref) => {
                self.parse_type_ref_parameter(tref, ty, required, description)
            }
            _ => Ok(HandlerParameter::PathOrQueryOrBody {
                schema: self.convert_to_json_schema(ty, span, true),
                required,
                description,
                span: *span,
            }),
        }
    }

    fn get_current_file(&mut self) -> Result<Rc<ParsedModule>> {
        match self.files.get_or_fetch_file(&self.current_file) {
            Some(it) => Ok(it),
            None => {
                self.errors.push(
                    self.build_error(
                        &DUMMY_SP,
                        DiagnosticInfoMessage::CannotFindFileWhenConvertingToSchema(
                            self.current_file.clone(),
                        ),
                    )
                    .to_diag(None),
                );
                Err(anyhow!("cannot find file: {}", self.current_file.0))
            }
        }
    }
    fn visit_current_file(&mut self) -> Result<()> {
        let file = self.get_current_file()?;
        let module = file.module.module.clone();
        self.visit_module(&module);
        Ok(())
    }

    fn parse_arrow_parameter(
        &mut self,
        param: &Pat,
        parent_span: &Span,
    ) -> Result<Vec<(String, HandlerParameter)>> {
        match param {
            Pat::Ident(BindingIdent { id, type_ann }) => {
                if type_ann.is_none() {
                    return self.error(
                        &id.span,
                        DiagnosticInfoMessage::ParameterIdentMustHaveTypeAnnotation,
                    );
                }

                let comments = self.get_current_file()?.comments.get_leading(id.span.lo);
                let description =
                    comments.and_then(|it| self.parse_description_comment(it, &id.span));
                let ty = self.assert_and_extract_type_from_ann(type_ann, &id.span);
                let param = self.parse_parameter_type(&ty, !id.optional, description, &id.span)?;
                Ok(vec![(id.sym.to_string(), param)])
            }
            Pat::Rest(RestPat { span, .. })
            | Pat::Array(ArrayPat { span, .. })
            | Pat::Object(ObjectPat { span, .. })
            | Pat::Assign(AssignPat { span, .. })
            | Pat::Invalid(Invalid { span, .. }) => {
                self.error(span, DiagnosticInfoMessage::ParameterPatternNotSupported)
            }
            Pat::Expr(_) => {
                self.error(
                    parent_span,
                    DiagnosticInfoMessage::ParameterPatternNotSupported,
                )
            }
        }
    }

    fn get_endpoint_comments(&mut self, key: &PropName) -> Result<EndpointComments> {
        let comments = self
            .get_current_file()?
            .comments
            .get_leading(get_prop_name_span(key).lo);

        let mut endpoint_comments = EndpointComments {
            summary: None,
            description: None,
        };
        if let Some(comments) = comments {
            self.parse_endpoint_comments(&mut endpoint_comments, comments);
        }
        Ok(endpoint_comments)
    }
    fn validate_handler_func(
        &mut self,
        is_generator: bool,
        type_params: &Option<Box<TsTypeParamDecl>>,
        span: &Span,
    ) -> Result<()> {
        if is_generator {
            return self.error(span, DiagnosticInfoMessage::HandlerCannotBeGenerator);
        }
        if type_params.is_some() {
            return self.error(span, DiagnosticInfoMessage::HandlerCannotHaveTypeParameters);
        }

        Ok(())
    }
    fn method_from_func_expr(&mut self, key: &PropName, func: &FnExpr) -> Result<FnHandler> {
        let FnExpr { function, .. } = func;
        let Function {
            params,
            span: parent_span,
            is_generator,
            type_params,
            return_type,
            ..
        } = &**function;
        self.validate_handler_func(*is_generator, type_params, parent_span)?;

        let endpoint_comments = self.get_endpoint_comments(key)?;

        let return_type = self.assert_and_extract_type_from_ann(return_type, parent_span);
        let parameters = params
            .iter()
            .map(|it| self.parse_arrow_parameter(&it.pat, parent_span))
            .collect::<Result<Vec<_>>>()?;
        let parameters = parameters.into_iter().flatten().collect();
        let e = FnHandler {
            method_kind: self.parse_method_kind(key)?,
            parameters,
            summary: endpoint_comments.summary,
            description: endpoint_comments.description,
            return_type: self.convert_to_json_schema(
                maybe_extract_promise(&return_type),
                parent_span,
                true,
            ),
            span: *parent_span,
        };

        Ok(e)
    }

    fn assert_and_extract_type_from_ann(
        &mut self,
        return_type: &Option<Box<TsTypeAnn>>,
        span: &Span,
    ) -> TsType {
        match return_type.as_ref() {
            Some(t) => (*t.type_ann).clone(),
            None => {
                self.push_error(span, DiagnosticInfoMessage::HandlerMustAnnotateReturnType);

                TsType::TsKeywordType(TsKeywordType {
                    span: DUMMY_SP,
                    kind: TsKeywordTypeKind::TsAnyKeyword,
                })
            }
        }
    }

    fn parse_method_kind(&mut self, key: &PropName) -> Result<MethodKind> {
        match key {
            PropName::Ident(Ident { sym, span, .. }) => match sym.to_string().as_str() {
                "get" => Ok(MethodKind::Get),
                "post" => Ok(MethodKind::Post),
                "put" => Ok(MethodKind::Put),
                "delete" => Ok(MethodKind::Delete),
                "patch" => Ok(MethodKind::Patch),
                "options" => Ok(MethodKind::Options),
                "use" => Ok(MethodKind::Use),
                _ => {
                    self.error(span, DiagnosticInfoMessage::NotAnHttpMethod)
                }
            },
            _ => {
                let span = get_prop_name_span(key);
                self.error(&span, DiagnosticInfoMessage::NotAnHttpMethod)
            }
        }
    }

    fn method_from_arrow_expr(&mut self, key: &PropName, arrow: &ArrowExpr) -> Result<FnHandler> {
        let ArrowExpr {
            params,
            is_generator,
            type_params,
            return_type,
            span: parent_span,
            ..
        } = arrow;
        self.validate_handler_func(*is_generator, type_params, parent_span)?;

        let endpoint_comments = self.get_endpoint_comments(key)?;

        let ret_ty = self.assert_and_extract_type_from_ann(return_type, parent_span);
        let e = FnHandler {
            method_kind: self.parse_method_kind(key)?,
            parameters: params
                .iter()
                .map(|it| self.parse_arrow_parameter(it, parent_span))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect(),
            summary: endpoint_comments.summary,
            description: endpoint_comments.description,
            return_type: self.convert_to_json_schema(
                maybe_extract_promise(&ret_ty),
                parent_span,
                true,
            ),
            span: *parent_span,
        };

        Ok(e)
    }

    fn get_prop_span(prop: &Prop) -> Span {
        match &prop {
            Prop::Shorthand(Ident { span, .. }) => *span,
            Prop::KeyValue(KeyValueProp { key, .. }) => get_prop_name_span(key),
            Prop::Assign(AssignProp { ref key, .. }) => key.span,
            Prop::Getter(GetterProp { span, .. }) => *span,
            Prop::Setter(SetterProp { span, .. }) => *span,
            Prop::Method(MethodProp { key, .. }) => get_prop_name_span(key),
        }
    }

    fn endpoints_from_method_map(&mut self, prop: &Prop) -> Vec<FnHandler> {
        if let Prop::KeyValue(KeyValueProp { key, value }) = prop {
            let method_kind = self.parse_method_kind(key);
            if let Ok(MethodKind::Use) = method_kind {
                return vec![FnHandler {
                    summary: None,
                    description: None,
                    parameters: vec![],
                    return_type: JsonSchema::Any,
                    method_kind: MethodKind::Use,
                    span: Self::get_prop_span(prop),
                }];
            }

            if let Expr::Arrow(arr) = &**value {
                match self.method_from_arrow_expr(key, arr) {
                    Ok(it) => return vec![it],
                    Err(_) => return vec![],
                }
            }

            if let Expr::Fn(func) = &**value {
                match self.method_from_func_expr(key, func) {
                    Ok(it) => return vec![it],
                    Err(_) => return vec![],
                }
            }
        }
        let span = Self::get_prop_span(prop);
        self.push_error(&span, DiagnosticInfoMessage::NotAnObjectWithMethodKind);

        vec![]
    }

    fn validate_pattern_was_consumed(
        &mut self,
        e: FnHandler,
        pattern: &ParsedPattern,
    ) -> FnHandler {
        for path_param in &pattern.path_params {
            // make sure some param exist for it
            let found = e.parameters.iter().find(|(key, _)| key == path_param);
            if found.is_none() {
                self.push_error(
                    &e.span,
                    DiagnosticInfoMessage::UnmatchedPathParameter(path_param.to_string()),
                );
            }
        }
        e
    }
    fn endpoints_from_prop(&mut self, prop: &Prop) -> Result<PathHandlerMap> {
        if let Prop::KeyValue(KeyValueProp { key, value }) = prop {
            let pattern = self.parse_key(key)?;
            if let Expr::Object(obj) = &**value {
                let handlers = obj
                    .props
                    .iter()
                    .flat_map(|it| match it {
                        PropOrSpread::Spread(it) => {
                            self.push_error(
                                &it.dot3_token,
                                DiagnosticInfoMessage::PropSpreadIsNotSupportedOnMethodMap,
                            );

                            vec![]
                        }
                        PropOrSpread::Prop(prop) => self
                            .endpoints_from_method_map(prop)
                            .into_iter()
                            .map(|handler| self.validate_pattern_was_consumed(handler, &pattern))
                            .collect(),
                    })
                    .collect();

                return Ok(PathHandlerMap { pattern, handlers });
            }
        }

        let span = Self::get_prop_span(prop);
        self.push_error(&span, DiagnosticInfoMessage::NotAnObjectWithMethodKind);

        Err(anyhow!("not an object"))
    }

    #[allow(clippy::to_string_in_format_args)]
    fn parse_export_default_comments(&mut self, comments: Vec<Comment>) {
        for c in comments {
            if c.kind == CommentKind::Block {
                let s = c.text;
                let parsed =
                    jsdoc::parse(Input::new(BytePos(0), BytePos(s.as_bytes().len() as _), &s));
                match parsed {
                    Ok((rest, parsed)) => {
                        self.info.description = Some(trim_description_comments(
                            parsed.description.value.to_string(),
                        ));
                        if rest.len() > 0 {
                            self.info.description = Some(format!(
                                "{}\n{}",
                                self.info.description.as_ref().unwrap_or(&String::new()),
                                rest.to_string()
                            ));
                        }
                        for tag_item in parsed.tags {
                            match tag_item.tag {
                                Tag::Version(VersionTag { value, .. }) => {
                                    self.info.version = Some(value.value.to_string());
                                }
                                Tag::Unknown(UnknownTag { extras, .. }) => {
                                    match &*tag_item.tag_name.value {
                                        "title" => {
                                            self.info.title = Some(extras.value.to_string());
                                        }
                                        tag => self.push_error(
                                            &c.span,
                                            DiagnosticInfoMessage::UnknownJsDocTagOfTypeUnknown(
                                                tag.to_string(),
                                            ),
                                        ),
                                    }
                                }
                                _v => self.push_error(
                                    &c.span,
                                    DiagnosticInfoMessage::UnknownJsDocTagOnRouter(
                                        tag_item.tag_name.value.to_string(),
                                    ),
                                ),
                            }
                        }
                    }
                    Err(_) => self.push_error(
                        &c.span,
                        DiagnosticInfoMessage::CannotParseJsDocExportDefault,
                    ),
                }
            }
        }
    }

    fn extract_one_built_decoder(&mut self, prop: &TsTypeElement) -> Result<BuiltDecoder> {
        match prop {
            TsTypeElement::TsPropertySignature(TsPropertySignature {
                key,
                type_ann,
                type_params,
                span,
                ..
            }) => {
                if type_params.is_some() {
                    return self.error(span, DiagnosticInfoMessage::GenericDecoderIsNotSupported);
                }

                let key = match &**key {
                    Expr::Ident(ident) => ident.sym.to_string(),
                    _ => {
                        return self.error(span, DiagnosticInfoMessage::InvalidDecoderKey);
                    }
                };
                match type_ann.as_ref().map(|it| &it.type_ann) {
                    Some(ann) => Ok(BuiltDecoder {
                        exported_name: key,
                        schema: self.convert_to_json_schema(ann, span, false),
                    }),
                    None => self.error(span, DiagnosticInfoMessage::DecoderMustHaveTypeAnnotation),
                }
            }
            TsTypeElement::TsGetterSignature(TsGetterSignature { span, .. })
            | TsTypeElement::TsSetterSignature(TsSetterSignature { span, .. })
            | TsTypeElement::TsMethodSignature(TsMethodSignature { span, .. })
            | TsTypeElement::TsIndexSignature(TsIndexSignature { span, .. })
            | TsTypeElement::TsCallSignatureDecl(TsCallSignatureDecl { span, .. })
            | TsTypeElement::TsConstructSignatureDecl(TsConstructSignatureDecl { span, .. }) => {
                self.error(span, DiagnosticInfoMessage::InvalidDecoderProperty)
            }
        }
    }
    fn extract_built_decoders_from_call(
        &mut self,
        params: &TsTypeParamInstantiation,
    ) -> Result<Vec<BuiltDecoder>> {
        match params.params.split_first() {
            Some((head, tail)) => {
                if !tail.is_empty() {
                    return self.error(
                        &params.span,
                        DiagnosticInfoMessage::TooManyTypeParamsOnDecoder,
                    );
                }
                match &**head {
                    TsType::TsTypeLit(TsTypeLit { members, .. }) => members
                        .iter()
                        .map(|prop| self.extract_one_built_decoder(prop))
                        .collect(),
                    _ => self.error(
                        &params.span,
                        DiagnosticInfoMessage::DecoderShouldBeObjectWithTypesAndNames,
                    ),
                }
            }
            None => self.error(
                &params.span,
                DiagnosticInfoMessage::TooFewTypeParamsOnDecoder,
            ),
        }
    }
}

impl<'a, R: FileManager> Visit for ExtractExportDefaultVisitor<'a, R> {
    fn visit_call_expr(&mut self, n: &CallExpr) {
        match n.callee {
            Callee::Super(_) => {}
            Callee::Import(_) => {}
            Callee::Expr(ref expr) => match &**expr {
                Expr::Ident(Ident { sym, span, .. }) => {
                    if sym == "buildParsers" {
                        match self.built_decoders {
                            Some(_) => self
                                .push_error(span, DiagnosticInfoMessage::TwoCallsToBuildParsers),
                            None => match n.type_args {
                                Some(ref params) => {
                                    match self.extract_built_decoders_from_call(params.as_ref()) {
                                        Ok(x) => self.built_decoders = Some(x),
                                        Err(_) => {}
                                    }
                                }
                                None => {
                                    // TS will catch the issue
                                }
                            },
                        }
                    }
                }
                _ => {}
            },
        }
    }
    fn visit_export_default_expr(&mut self, n: &ExportDefaultExpr) {
        match self.get_current_file() {
            Ok(file) => {
                let comments = file.comments.get_leading(n.span.lo);
                if let Some(comments) = comments {
                    self.parse_export_default_comments(comments);
                }
                if let Expr::Object(lit) = &*n.expr {
                    self.found_default_export = true;
                    for prop in &lit.props {
                        match prop {
                            PropOrSpread::Prop(prop) => {
                                let method = self.endpoints_from_prop(prop);
                                if let Ok(method) = method {
                                    self.handlers.push(method)
                                };
                            }
                            PropOrSpread::Spread(SpreadElement { dot3_token, .. }) => {
                                self.push_error(
                                    dot3_token,
                                    DiagnosticInfoMessage::RestOnRouterDefaultExportNotSupportedYet,
                                );
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }
}

pub enum FunctionParameterIn {
    Path,
    Query,
    Body,
    InvalidComplexPathParameter,
}

fn is_type_simple(it: &JsonSchema, components: &Vec<Definition>) -> bool {
    match it {
        JsonSchema::OpenApiResponseRef(r) | JsonSchema::Ref(r) => {
            let def = components
                .iter()
                .find(|it| &it.name == r)
                .expect("can always find ref in json schema at this point");
            is_type_simple(&def.schema, components)
        }
        JsonSchema::AnyOf(vs) => vs.iter().all(|it| is_type_simple(it, components)),
        JsonSchema::Null
        | JsonSchema::Boolean
        | JsonSchema::String
        | JsonSchema::Number
        | JsonSchema::StringWithFormat(_)
        | JsonSchema::Const(_) => true,
        JsonSchema::AllOf(_)
        | JsonSchema::Any
        | JsonSchema::Object { .. }
        | JsonSchema::Array(_)
        | JsonSchema::Tuple { .. } => false,
        JsonSchema::Error => true,
    }
}

pub fn operation_parameter_in_path_or_query_or_body(
    name: &str,
    pattern: &ParsedPattern,
    schema: &JsonSchema,
    components: &Vec<Definition>,
) -> FunctionParameterIn {
    // if name is in pattern return path
    if pattern.path_params.contains(&name.to_string()) {
        if is_type_simple(schema, components) {
            FunctionParameterIn::Path
        } else {
            FunctionParameterIn::InvalidComplexPathParameter
        }
    } else if is_type_simple(schema, components) {
        FunctionParameterIn::Query
    } else {
        FunctionParameterIn::Body
    }
}

struct EndpointToPath<'a, R: FileManager> {
    files: &'a mut R,
    errors: Vec<Diagnostic>,
    components: &'a Vec<Definition>,
    current_file: BffFileName,
}

impl<'a, R: FileManager> EndpointToPath<'a, R> {
    fn push_error(
        &mut self,
        span: &Span,
        info_msg: DiagnosticInfoMessage,
        parent_msg: Option<DiagnosticParentMessage>,
    ) {
        let file = self.files.get_or_fetch_file(&self.current_file);
        match file {
            Some(file) => {
                let (loc_lo, loc_hi) =
                    span_to_loc(span, &file.module.source_map, file.module.fm.end_pos);

                let err = DiagnosticInformation::KnownFile {
                    message: info_msg,
                    file_name: self.current_file.clone(),
                    loc_lo,
                    loc_hi,
                };
                self.errors.push(err.to_diag(parent_msg));
            }
            None => {
                let err = DiagnosticInformation::UnfoundFile {
                    message: info_msg,
                    current_file: self.current_file.clone(),
                };
                self.errors.push(err.to_diag(parent_msg));
            }
        }
    }

    fn endpoint_to_operation_object(
        &mut self,
        endpoint: &FnHandler,
        pattern: &ParsedPattern,
    ) -> OperationObject {
        let mut parameters: Vec<ParameterObject> = vec![];
        let mut json_request_body: Option<JsonRequestBody> = None;

        if let Some((first_param, rest_param)) = endpoint.parameters.split_first() {
            match first_param.1 {
                HandlerParameter::Context(_) => {}
                HandlerParameter::PathOrQueryOrBody { span, .. }
                | HandlerParameter::HeaderOrCookie { span, .. } => {
                    self.push_error(
                        &span,
                        DiagnosticInfoMessage::ContextParameterMustBeFirst,
                        Some(DiagnosticParentMessage::InvalidContextPosition),
                    );
                }
            }
            for (_, param) in rest_param {
                match param {
                    HandlerParameter::Context(span) => {
                        self.push_error(
                            span,
                            DiagnosticInfoMessage::ContextInvalidAtThisPosition,
                            Some(DiagnosticParentMessage::InvalidContextPosition),
                        );
                    }
                    _ => {}
                }
            }
        }

        for (key, param) in endpoint.parameters.iter() {
            match param {
                HandlerParameter::PathOrQueryOrBody {
                    schema,
                    required,
                    description,
                    span,
                    ..
                } => {
                    match operation_parameter_in_path_or_query_or_body(
                        key,
                        pattern,
                        schema,
                        self.components,
                    ) {
                        FunctionParameterIn::Path => parameters.push(ParameterObject {
                            in_: ParameterIn::Path,
                            name: key.clone(),
                            required: *required,
                            description: description.clone(),
                            schema: schema.clone(),
                        }),
                        FunctionParameterIn::Query => parameters.push(ParameterObject {
                            in_: ParameterIn::Query,
                            name: key.clone(),
                            required: *required,
                            description: description.clone(),
                            schema: schema.clone(),
                        }),
                        FunctionParameterIn::Body => {
                            if json_request_body.is_some() {
                                self.push_error(
                                    span,
                                    DiagnosticInfoMessage::InferringTwoParamsAsRequestBody,
                                    None,
                                );
                            }

                            json_request_body = Some(JsonRequestBody {
                                schema: schema.clone(),
                                description: description.clone(),
                                required: *required,
                            });
                        }
                        FunctionParameterIn::InvalidComplexPathParameter => {
                            self.push_error(
                                span,
                                DiagnosticInfoMessage::ComplexPathParameterNotSupported,
                                Some(DiagnosticParentMessage::ComplexPathParam),
                            );
                        }
                    }
                }
                HandlerParameter::HeaderOrCookie {
                    required,
                    description,
                    kind,
                    schema,
                    ..
                } => parameters.push(ParameterObject {
                    in_: match kind {
                        HeaderOrCookie::Header => ParameterIn::Header,
                        HeaderOrCookie::Cookie => ParameterIn::Cookie,
                    },
                    name: key.clone(),
                    required: *required,
                    description: description.clone(),
                    schema: schema.clone(),
                }),
                HandlerParameter::Context(_) => {}
            };
        }
        OperationObject {
            summary: endpoint.summary.clone(),
            description: endpoint.description.clone(),
            parameters,
            json_response_body: endpoint.return_type.clone(),
            json_request_body,
        }
    }

    fn endpoints_to_paths(
        &mut self,
        endpoints: &Vec<PathHandlerMap>,
    ) -> Vec<open_api_ast::ApiPath> {
        endpoints
            .iter()
            .map(|it| {
                let mut path = open_api_ast::ApiPath::from_pattern(&it.pattern.open_api_pattern);
                for endpoint in &it.handlers {
                    let kind = endpoint.method_kind;
                    let op = Some(self.endpoint_to_operation_object(endpoint, &it.pattern));
                    match kind {
                        MethodKind::Get => path.get = op,
                        MethodKind::Post => path.post = op,
                        MethodKind::Put => path.put = op,
                        MethodKind::Delete => path.delete = op,
                        MethodKind::Patch => path.patch = op,
                        MethodKind::Options => path.options = op,
                        MethodKind::Use => {}
                    }
                }
                path
            })
            .collect()
    }
}

pub struct PathHandlerMap {
    pub pattern: ParsedPattern,
    pub handlers: Vec<FnHandler>,
}
pub struct ExtractResult {
    pub errors: Vec<Diagnostic>,
    pub open_api: OpenApi,
    pub entry_file_name: BffFileName,
    pub handlers: Vec<PathHandlerMap>,
    pub built_decoders: Option<Vec<BuiltDecoder>>,
    pub components: Vec<Definition>,
}

fn visit_extract<R: FileManager>(
    files: &mut R,
    current_file: BffFileName,
) -> (
    Vec<PathHandlerMap>,
    Vec<Definition>,
    Vec<Diagnostic>,
    Info,
    Option<Vec<BuiltDecoder>>,
    HashSet<String>,
) {
    let mut visitor = ExtractExportDefaultVisitor::new(files, current_file.clone());

    let _ = visitor.visit_current_file();

    if !visitor.found_default_export {
        visitor.errors.push(
            DiagnosticInformation::UnfoundFile {
                message: DiagnosticInfoMessage::CouldNotFindDefaultExport,
                current_file: current_file.clone(),
            }
            .to_diag(None),
        )
    }

    (
        visitor.handlers,
        visitor.components,
        visitor.errors,
        visitor.info,
        visitor.built_decoders,
        visitor.public_definitions,
    )
}

pub fn extract_schema<R: FileManager>(
    files: &mut R,
    entry_file_name: BffFileName,
) -> ExtractResult {
    let (handlers, components, errors, info, built_decoders, public_definitions) =
        visit_extract(files, entry_file_name.clone());

    let mut transformer = EndpointToPath {
        errors,
        components: &components,
        current_file: entry_file_name.clone(),
        files,
    };
    let paths = transformer.endpoints_to_paths(&handlers);
    let errors = transformer.errors;
    let open_api = OpenApi {
        info,
        components: public_definitions.into_iter().collect(),
        paths,
    };

    ExtractResult {
        handlers,
        open_api,
        entry_file_name,
        errors,
        built_decoders,
        components,
    }
}
