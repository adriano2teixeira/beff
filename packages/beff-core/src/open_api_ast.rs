use core::fmt;

use indexmap::IndexMap;
use swc_ecma_ast::Expr;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Optionality<T> {
    Optional(T),
    Required(T),
}

impl<T> Optionality<T> {
    pub fn inner(&self) -> &T {
        match self {
            Optionality::Optional(t) | Optionality::Required(t) => t,
        }
    }
    pub fn inner_move(self) -> T {
        match self {
            Optionality::Optional(t) | Optionality::Required(t) => t,
        }
    }
    pub fn is_required(&self) -> bool {
        match self {
            Optionality::Optional(_) => false,
            Optionality::Required(_) => true,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(IndexMap<String, Json>),
}

impl Json {
    #[must_use]
    pub fn to_js(self) -> Js {
        match self {
            Json::Null => Js::Null,
            Json::Bool(b) => Js::Bool(b),
            Json::Number(n) => Js::Number(n),
            Json::String(s) => Js::String(s),
            Json::Array(arr) => Js::Array(arr.into_iter().map(Json::to_js).collect()),
            Json::Object(obj) => Js::Object(obj.into_iter().map(|(k, v)| (k, v.to_js())).collect()),
        }
    }

    pub fn object(vs: Vec<(String, Json)>) -> Self {
        Self::Object(vs.into_iter().collect())
    }

    pub fn to_serde(&self) -> serde_json::Value {
        match self {
            Json::Null => serde_json::Value::Null,
            Json::Bool(b) => serde_json::Value::Bool(*b),
            Json::Number(n) => serde_json::Value::Number(
                serde_json::Number::from_f64(*n)
                    .expect("should be possible to convert f64 to json number"),
            ),
            Json::String(s) => serde_json::Value::String(s.clone()),
            Json::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|it| it.to_serde()).collect::<Vec<_>>())
            }
            Json::Object(obj) => serde_json::Value::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.to_serde()))
                    .collect::<serde_json::Map<_, _>>(),
            ),
        }
    }

    pub fn from_serde(it: &serde_json::Value) -> Json {
        match it {
            serde_json::Value::Null => Json::Null,
            serde_json::Value::Bool(v) => Json::Bool(*v),
            serde_json::Value::Number(v) => Json::Number(v.as_f64().unwrap()),
            serde_json::Value::String(st) => Json::String(st.clone()),
            serde_json::Value::Array(vs) => Json::Array(vs.iter().map(Json::from_serde).collect()),
            serde_json::Value::Object(vs) => Json::Object(
                vs.iter()
                    .map(|(k, v)| (k.clone(), Json::from_serde(v)))
                    .collect(),
            ),
        }
    }
}

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string_pretty(&self.to_serde())
                .expect("should be possible to serialize json")
        )
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Js {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Js>),
    Object(IndexMap<String, Js>),
    Decoder {
        name_on_errors: Option<String>,
        schema: JsonSchema,
        required: bool,
    },
    Coercer(JsonSchema),
    Expr(Expr),
}

fn resolve_schema(schema: JsonSchema, components: &Vec<Validator>) -> JsonSchema {
    match schema {
        JsonSchema::Ref(name) => match components.iter().find(|it| it.name == name) {
            Some(def) => resolve_schema(def.schema.clone(), components),
            None => unreachable!("everything should be resolved when printing"),
        },
        _ => schema,
    }
}

impl Js {
    pub fn object(vs: Vec<(String, Js)>) -> Self {
        Self::Object(vs.into_iter().collect())
    }

    pub fn coercer(schema: JsonSchema, components: &Vec<Validator>) -> Self {
        Self::Coercer(resolve_schema(schema, components))
    }

    pub fn named_decoder(name: String, schema: JsonSchema, required: bool) -> Self {
        Self::Decoder {
            name_on_errors: Some(name),
            schema,
            required,
        }
    }
    pub fn anon_decoder(schema: JsonSchema, required: bool) -> Self {
        Self::Decoder {
            name_on_errors: None,
            schema,
            required,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum JsonSchema {
    Null,
    Boolean,
    String,
    StringWithFormat(String),
    Number,
    Any,
    Object(IndexMap<String, Optionality<JsonSchema>>),
    Array(Box<JsonSchema>),
    Tuple {
        prefix_items: Vec<JsonSchema>,
        items: Option<Box<JsonSchema>>,
    },
    Ref(String),
    OpenApiResponseRef(String),
    AnyOf(Vec<JsonSchema>),
    AllOf(Vec<JsonSchema>),
    Const(Json),
    Error,
}

impl JsonSchema {
    pub fn object(vs: Vec<(String, Optionality<JsonSchema>)>) -> Self {
        Self::Object(vs.into_iter().collect())
    }
    pub fn required(self) -> Optionality<JsonSchema> {
        Optionality::Required(self)
    }
    pub fn optional(self) -> Optionality<JsonSchema> {
        Optionality::Optional(self)
    }
}

#[derive(Debug)]
pub struct Info {
    pub title: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug)]
pub enum ParameterIn {
    Query,
    Header,
    Path,
}

impl fmt::Display for ParameterIn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParameterIn::Query => write!(f, "query"),
            ParameterIn::Header => write!(f, "header"),
            ParameterIn::Path => write!(f, "path"),
        }
    }
}

#[derive(Debug)]
pub struct ParameterObject {
    pub name: String,
    pub in_: ParameterIn,
    pub description: Option<String>,
    pub required: bool,
    pub schema: JsonSchema,
}

#[derive(Debug)]
pub struct JsonRequestBody {
    pub description: Option<String>,
    pub schema: JsonSchema,
    pub required: bool,
}

#[derive(Debug)]
pub struct OperationObject {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParameterObject>,
    pub json_response_body: JsonSchema,
    pub json_request_body: Option<JsonRequestBody>,
}

#[derive(Debug)]
pub struct ApiPath {
    pub pattern: String,
    pub get: Option<OperationObject>,
    pub post: Option<OperationObject>,
    pub put: Option<OperationObject>,
    pub delete: Option<OperationObject>,
    pub patch: Option<OperationObject>,
    pub options: Option<OperationObject>,
}

impl ApiPath {
    #[must_use]
    pub fn from_pattern(pattern: &str) -> Self {
        Self {
            pattern: pattern.into(),
            get: None,
            post: None,
            put: None,
            delete: None,
            patch: None,
            options: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Validator {
    pub name: String,
    pub schema: JsonSchema,
}

#[derive(Debug)]
pub struct OpenApi {
    pub info: Info,
    pub paths: Vec<ApiPath>,
    pub components: Vec<String>,
}
