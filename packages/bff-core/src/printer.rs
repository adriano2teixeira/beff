use anyhow::Result;
use serde::{Deserialize, Serialize};
use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    ArrayLit, BindingIdent, Bool, Decl, Expr, ExprOrSpread, FnDecl, FnExpr, Ident, KeyValueProp,
    Lit, ModuleItem, Null, Number, ObjectLit, Pat, Prop, PropName, PropOrSpread, Stmt, Str,
    VarDecl, VarDeclKind, VarDeclarator,
};

use crate::api_extractor::{
    operation_parameter_in_path_or_query_or_body, BuiltDecoder, ExtractResult, FunctionParameterIn,
    HandlerParameter, HeaderOrCookie, ParsedPattern, PathHandlerMap,
};
use crate::decoder;
use crate::emit::emit_module;
use crate::open_api_ast::{self, Definition, Js, Json, JsonSchema, OpenApi};

pub trait ToExpr {
    fn to_expr(self) -> Expr;
}
impl ToExpr for Json {
    fn to_expr(self) -> Expr {
        match self {
            Json::Null => Expr::Lit(Lit::Null(Null { span: DUMMY_SP })),
            Json::Bool(v) => Expr::Lit(Lit::Bool(Bool {
                span: DUMMY_SP,
                value: v,
            })),
            Json::Number(n) => Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: n,
                raw: None,
            })),
            Json::String(v) => Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: v.into(),
                raw: None,
            })),
            Json::Array(els) => Expr::Array(ArrayLit {
                span: DUMMY_SP,
                elems: els
                    .into_iter()
                    .map(|it| {
                        Some(ExprOrSpread {
                            spread: None,
                            expr: Box::new(it.to_expr()),
                        })
                    })
                    .collect(),
            }),
            Json::Object(kvs) => Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: kvs
                    .into_iter()
                    .map(|(key, value)| {
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Str(Str {
                                span: DUMMY_SP,
                                value: key.into(),
                                raw: None,
                            }),
                            value: Box::new(value.to_expr()),
                        })))
                    })
                    .collect(),
            }),
        }
    }
}

trait ToJson {
    fn to_json(self) -> Json;
}
trait ToJsonKv {
    fn to_json_kv(self) -> Vec<(String, Json)>;
}

impl ToJson for JsonSchema {
    #[allow(clippy::cast_precision_loss)]
    fn to_json(self) -> Json {
        match self {
            JsonSchema::String => {
                Json::Object(vec![("type".into(), Json::String("string".into()))])
            }
            JsonSchema::Object { values } => {
                Json::Object(vec![
                    //
                    ("type".into(), Json::String("object".into())),
                    (
                        "required".into(),
                        //
                        Json::Array(
                            values
                                .iter()
                                .filter(|(_k, v)| v.is_required())
                                .map(|(k, _v)| Json::String(k.clone()))
                                .collect(),
                        ),
                    ),
                    (
                        "properties".into(),
                        //
                        Json::Object(
                            values
                                .into_iter()
                                .map(|(k, v)| (k, v.inner_move().to_json()))
                                .collect(),
                        ),
                    ),
                ])
            }
            JsonSchema::Array(typ) => {
                Json::Object(vec![
                    //
                    ("type".into(), Json::String("array".into())),
                    ("items".into(), (*typ).to_json()),
                ])
            }
            JsonSchema::Boolean => {
                Json::Object(vec![("type".into(), Json::String("boolean".into()))])
            }
            JsonSchema::Number => {
                Json::Object(vec![("type".into(), Json::String("number".into()))])
            }
            JsonSchema::Any => Json::Object(vec![]),
            JsonSchema::Ref(reference) => Json::Object(vec![(
                "$ref".into(),
                Json::String(format!("#/components/schemas/{reference}")),
            )]),
            JsonSchema::ResponseRef(reference) => Json::Object(vec![(
                "$ref".into(),
                Json::String(format!("#/components/responses/{reference}")),
            )]),
            JsonSchema::Null => Json::Object(vec![("type".into(), Json::String("null".into()))]),
            JsonSchema::AnyOf(types) => {
                let all_literals = types.iter().all(|it| matches!(it, JsonSchema::Const(_)));
                if all_literals {
                    let vs = types
                        .into_iter()
                        .map(|it| match it {
                            JsonSchema::Const(e) => e,
                            _ => unreachable!("should have been caught by all_literals check"),
                        })
                        .collect();
                    Json::Object(vec![("enum".into(), Json::Array(vs))])
                } else {
                    let vs = types.into_iter().map(ToJson::to_json).collect();
                    Json::Object(vec![("anyOf".into(), Json::Array(vs))])
                }
            }
            JsonSchema::AllOf(types) => Json::Object(vec![(
                "allOf".into(),
                Json::Array(types.into_iter().map(ToJson::to_json).collect()),
            )]),

            JsonSchema::Tuple {
                prefix_items,
                items,
            } => {
                let mut v = vec![
                    //
                    ("type".into(), Json::String("array".into())),
                ];
                let len_f = prefix_items.len() as f64;
                if !prefix_items.is_empty() {
                    v.push((
                        "prefixItems".into(),
                        Json::Array(prefix_items.into_iter().map(ToJson::to_json).collect()),
                    ));
                }
                if let Some(ty) = items {
                    v.push(("items".into(), ty.to_json()));
                } else {
                    v.push(("minItems".into(), Json::Number(len_f)));
                    v.push(("maxItems".into(), Json::Number(len_f)));
                }
                Json::Object(v)
            }
            JsonSchema::Const(val) => Json::Object(vec![("const".into(), val)]),
        }
    }
}
impl ToJson for open_api_ast::ParameterObject {
    fn to_json(self) -> Json {
        let mut v = vec![];
        v.push(("name".into(), Json::String(self.name)));
        v.push(("in".into(), Json::String(self.in_.to_string())));
        if let Some(desc) = self.description {
            v.push(("description".into(), Json::String(desc)));
        }
        v.push(("required".into(), Json::Bool(self.required)));
        v.push(("schema".into(), self.schema.to_json()));
        Json::Object(v)
    }
}
impl ToJson for open_api_ast::JsonRequestBody {
    fn to_json(self) -> Json {
        let mut v = vec![];
        if let Some(desc) = self.description {
            v.push(("description".into(), Json::String(desc)));
        }
        v.push(("required".into(), Json::Bool(self.required)));
        let content = Json::Object(vec![(
            "application/json".into(),
            Json::Object(vec![("schema".into(), self.schema.to_json())]),
        )]);
        v.push(("content".into(), content));
        Json::Object(v)
    }
}

fn error_response_ref(code: &str, reference: &str) -> (String, Json) {
    (
        code.into(),
        JsonSchema::ResponseRef(reference.to_owned()).to_json(),
    )
}
impl ToJson for open_api_ast::OperationObject {
    fn to_json(self) -> Json {
        let mut v = vec![];
        if let Some(summary) = self.summary {
            v.push(("summary".into(), Json::String(summary)));
        }
        if let Some(desc) = self.description {
            v.push(("description".into(), Json::String(desc)));
        }
        if let Some(body) = self.json_request_body {
            v.push(("requestBody".into(), body.to_json()));
        }
        v.push((
            "parameters".into(),
            Json::Array(self.parameters.into_iter().map(ToJson::to_json).collect()),
        ));
        v.push((
            "responses".into(),
            Json::Object(vec![
                (
                    "200".into(),
                    Json::Object(vec![
                        (
                            "description".into(),
                            Json::String("Successful Operation".into()),
                        ),
                        (
                            "content".into(),
                            Json::Object(vec![(
                                "application/json".into(),
                                Json::Object(vec![(
                                    "schema".into(),
                                    self.json_response_body.to_json(),
                                )]),
                            )]),
                        ),
                    ]),
                ),
                error_response_ref("422", "DecodeError"),
                error_response_ref("default", "UnexpectedError"),
            ]),
        ));

        Json::Object(v)
    }
}

impl ToJsonKv for open_api_ast::ApiPath {
    fn to_json_kv(self) -> Vec<(String, Json)> {
        let mut v = vec![];
        if let Some(get) = self.get {
            v.push(("get".into(), get.to_json()));
        }
        if let Some(post) = self.post {
            v.push(("post".into(), post.to_json()));
        }
        if let Some(put) = self.put {
            v.push(("put".into(), put.to_json()));
        }
        if let Some(delete) = self.delete {
            v.push(("delete".into(), delete.to_json()));
        }
        if let Some(patch) = self.patch {
            v.push(("patch".into(), patch.to_json()));
        }
        if let Some(options) = self.options {
            v.push(("options".into(), options.to_json()));
        }
        if v.is_empty() {
            return vec![];
        }
        vec![(self.pattern.clone(), Json::Object(v))]
    }
}
impl ToJsonKv for open_api_ast::Definition {
    fn to_json_kv(self) -> Vec<(String, Json)> {
        vec![(self.name.clone(), self.schema.to_json())]
    }
}
impl ToJson for open_api_ast::Info {
    fn to_json(self) -> Json {
        let mut v = vec![];
        if let Some(desc) = self.description {
            v.push(("description".into(), Json::String(desc)));
        }
        v.push((
            "title".into(),
            Json::String(self.title.unwrap_or("No title".to_owned())),
        ));
        v.push((
            "version".into(),
            Json::String(self.version.unwrap_or("0.0.0".to_owned())),
        ));
        Json::Object(v)
    }
}

fn error_response_schema() -> JsonSchema {
    JsonSchema::Object {
        values: vec![("message".to_string(), JsonSchema::String.required())],
    }
}

fn error_response(code: &str, description: &str) -> (String, Json) {
    (
        code.into(),
        Json::Object(vec![
            ("description".into(), Json::String(description.into())),
            (
                "content".into(),
                Json::Object(vec![(
                    "application/json".into(),
                    Json::Object(vec![("schema".into(), error_response_schema().to_json())]),
                )]),
            ),
        ]),
    )
}

impl ToJson for OpenApi {
    fn to_json(self) -> Json {
        let v = vec![
            //
            ("openapi".into(), Json::String("3.1.0".into())),
            ("info".into(), self.info.to_json()),
            (
                "paths".into(),
                Json::Object(
                    self.paths
                        .into_iter()
                        .flat_map(ToJsonKv::to_json_kv)
                        .collect(),
                ),
            ),
            (
                "components".into(),
                Json::Object(vec![
                    (
                        "schemas".into(),
                        Json::Object(
                            self.components
                                .into_iter()
                                .flat_map(ToJsonKv::to_json_kv)
                                .collect(),
                        ),
                    ),
                    (
                        "responses".into(),
                        Json::Object(vec![
                            error_response("DecodeError", "Invalid parameters or request body"),
                            error_response("UnexpectedError", "Unexpected Error"),
                        ]),
                    ),
                ]),
            ),
        ];
        Json::Object(v)
    }
}

fn param_to_js(
    name: &str,
    param: HandlerParameter,
    pattern: &ParsedPattern,
    components: &Vec<Definition>,
) -> Js {
    match param {
        HandlerParameter::PathOrQueryOrBody {
            schema, required, ..
        } => {
            match operation_parameter_in_path_or_query_or_body(&name, pattern, &schema, components)
            {
                FunctionParameterIn::Path => Js::Object(vec![
                    ("type".into(), Js::String("path".into())),
                    ("name".into(), Js::String(name.to_string())),
                    ("required".into(), Js::Bool(required)),
                    (
                        "validator".into(),
                        Js::Decoder(format!("Path Parameter \"{name}\""), schema.clone()),
                    ),
                    ("coercer".into(), Js::Coercer(schema)),
                ]),
                FunctionParameterIn::Query => Js::Object(vec![
                    ("type".into(), Js::String("query".into())),
                    ("name".into(), Js::String(name.to_string())),
                    ("required".into(), Js::Bool(required)),
                    (
                        "validator".into(),
                        Js::Decoder(format!("Query Parameter \"{name}\""), schema.clone()),
                    ),
                    ("coercer".into(), Js::Coercer(schema)),
                ]),
                FunctionParameterIn::Body => Js::Object(vec![
                    ("type".into(), Js::String("body".into())),
                    ("name".into(), Js::String(name.to_string())),
                    ("required".into(), Js::Bool(required)),
                    (
                        "validator".into(),
                        Js::Decoder(format!("Request Body"), schema.clone()),
                    ),
                ]),
                FunctionParameterIn::InvalidComplexPathParameter => {
                    unreachable!("will fail when extracting the json schema")
                }
            }
        }
        HandlerParameter::HeaderOrCookie {
            kind,
            schema,
            required,
            ..
        } => {
            let kind_name = match kind {
                HeaderOrCookie::Header => "Header Argument",
                HeaderOrCookie::Cookie => "Cookie Argument",
            };
            Js::Object(vec![
                //
                ("type".into(), Js::String(kind.to_string())),
                ("name".into(), Js::String(name.to_string())),
                ("required".into(), Js::Bool(required)),
                (
                    "validator".into(),
                    Js::Decoder(format!("{kind_name} \"{name}\""), schema.clone()),
                ),
                ("coercer".into(), Js::Coercer(schema)),
            ])
        }
        HandlerParameter::Context(_) => {
            Js::Object(vec![("type".into(), Js::String("context".into()))])
        }
    }
}

fn handlers_to_js(items: Vec<PathHandlerMap>, components: &Vec<Definition>) -> Js {
    Js::Array(
        items
            .into_iter()
            .flat_map(|it| {
                it.handlers
                    .into_iter()
                    .map(|handler| {
                        let ptn = &it.pattern.open_api_pattern;
                        let kind = handler.method_kind.to_string().to_uppercase();
                        let decoder_name = format!("[{kind}] {ptn}.response_body");
                        Js::Object(vec![
                            (
                                "method_kind".into(),
                                Js::String(handler.method_kind.to_string()),
                            ),
                            (
                                "params".into(),
                                Js::Array(
                                    handler
                                        .parameters
                                        .into_iter()
                                        .map(|(name, param)| {
                                            param_to_js(&name, param, &it.pattern, components)
                                        })
                                        .collect(),
                                ),
                            ),
                            (
                                "pattern".into(),
                                Js::String(it.pattern.open_api_pattern.clone()),
                            ),
                            (
                                "return_validator".into(),
                                Js::Decoder(decoder_name, handler.return_type),
                            ),
                        ])
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
    )
}

fn const_decl(name: &str, init: Expr) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Decl(Decl::Var(
        VarDecl {
            span: DUMMY_SP,
            kind: VarDeclKind::Const,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                name: Pat::Ident(BindingIdent {
                    id: Ident {
                        span: DUMMY_SP,
                        sym: name.into(),
                        optional: false,
                    },
                    type_ann: None,
                }),
                init: Some(Box::new(init)),
                definite: false,
            }],
        }
        .into(),
    )))
}

fn js_to_expr(it: Js, components: &Vec<Definition>) -> Expr {
    match it {
        Js::Decoder(name, schema) => Expr::Fn(FnExpr {
            ident: None,
            function: decoder::from_schema(&schema, &name).into(),
        }),
        Js::Coercer(schema) => {
            let func = crate::coercer::from_schema(&schema, components);
            Expr::Fn(FnExpr {
                ident: None,
                function: func.into(),
            })
        }
        Js::Null => Json::Null.to_expr(),
        Js::Bool(it) => Json::Bool(it).to_expr(),
        Js::Number(it) => Json::Number(it).to_expr(),
        Js::String(it) => Json::String(it).to_expr(),
        Js::Array(els) => Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: els
                .into_iter()
                .map(|it| {
                    Some(ExprOrSpread {
                        spread: None,
                        expr: Box::new(js_to_expr(it, components)),
                    })
                })
                .collect(),
        }),
        Js::Object(kvs) => Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: kvs
                .into_iter()
                .map(|(key, value)| {
                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: PropName::Str(Str {
                            span: DUMMY_SP,
                            value: key.into(),
                            raw: None,
                        }),
                        value: Box::new(js_to_expr(value, components)),
                    })))
                })
                .collect(),
        }),
    }
}

#[derive(Serialize, Deserialize)]
pub struct WritableModules {
    pub js_server_data: String,
    pub json_schema: String,
    pub had_build_decoders_call: bool,
}

pub trait ToWritableModules {
    fn to_module(self) -> Result<WritableModules>;
}
fn build_decoders_expr(decs: &Vec<BuiltDecoder>) -> Js {
    Js::Object(
        decs.iter()
            .map(|it| {
                (
                    it.exported_name.clone(),
                    Js::Decoder(it.exported_name.clone(), it.schema.clone()),
                )
            })
            .collect(),
    )
}
impl ToWritableModules for ExtractResult {
    fn to_module(self) -> Result<WritableModules> {
        let mut js_server_data = vec![];

        for comp in &self.open_api.components {
            let name = format!("validate_{}", comp.name);
            let decoder_fn = decoder::from_schema(&comp.schema, &comp.name);
            let decoder_fn_decl = ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
                ident: Ident {
                    span: DUMMY_SP,
                    sym: name.into(),
                    optional: false,
                },
                declare: false,
                function: decoder_fn.into(),
            })));
            js_server_data.push(decoder_fn_decl);
        }

        let components = &self.open_api.components;

        let meta_expr = js_to_expr(handlers_to_js(self.handlers, &components), &components);
        js_server_data.push(const_decl("meta", meta_expr));

        let mut had_build_decoders_call = false;
        if let Some(ref it) = self.built_decoders {
            if !it.is_empty() {
                let build_decoders_expr = build_decoders_expr(it);
                js_server_data.push(const_decl(
                    "buildDecodersInput",
                    js_to_expr(build_decoders_expr, &components),
                ));
                had_build_decoders_call = true;
            }
        }

        Ok(WritableModules {
            js_server_data: emit_module(js_server_data)?,
            json_schema: self.open_api.to_json().to_string(),
            had_build_decoders_call,
        })
    }
}
