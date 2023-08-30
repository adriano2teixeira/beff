use core::fmt;

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
    Object(Vec<(String, Json)>),
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
            Json::Object(obj) => Js::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, v.to_js()))
                    .collect::<Vec<_>>(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Js {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Js>),
    Object(Vec<(String, Js)>),
    Decoder(String, JsonSchema),
    Coercer(JsonSchema),
}

#[derive(Debug, PartialEq, Clone)]
pub enum JsonSchema {
    Null,
    Boolean,
    String,
    Number,
    Any,
    // Not(Box<JsonSchema>),
    Object {
        values: Vec<(String, Optionality<JsonSchema>)>,
    },
    Array(Box<JsonSchema>),
    Tuple {
        prefix_items: Vec<JsonSchema>,
        items: Option<Box<JsonSchema>>,
    },
    Ref(String),
    AnyOf(Vec<JsonSchema>),
    AllOf(Vec<JsonSchema>),
    Const(Json),
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
    Cookie,
}

impl fmt::Display for ParameterIn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParameterIn::Query => write!(f, "query"),
            ParameterIn::Header => write!(f, "header"),
            ParameterIn::Path => write!(f, "path"),
            ParameterIn::Cookie => write!(f, "cookie"),
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
pub struct Definition {
    pub name: String,
    pub schema: JsonSchema,
}

#[derive(Debug)]
pub struct OpenApi {
    pub info: Info,
    pub paths: Vec<ApiPath>,
    pub components: Vec<Definition>,
}