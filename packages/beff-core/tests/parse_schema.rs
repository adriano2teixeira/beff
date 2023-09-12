#[cfg(test)]
mod tests {

    use beff_core::ast::{
        json::{Json, ToJson},
        json_schema::JsonSchema,
    };

    #[test]
    fn it_works() {
        let schemas = vec![
            JsonSchema::Null,
            JsonSchema::Boolean,
            JsonSchema::String,
            JsonSchema::StringWithFormat("password".into()),
            JsonSchema::Number,
            JsonSchema::Any,
            JsonSchema::object(vec![]),
            JsonSchema::object(vec![
                ("foo".into(), JsonSchema::String.optional()),
                ("bar".into(), JsonSchema::Number.required()),
                (
                    "baz".into(),
                    JsonSchema::object(vec![
                        ("foo".into(), JsonSchema::String.optional()),
                        ("bar".into(), JsonSchema::Number.required()),
                        ("baz".into(), JsonSchema::Number.required()),
                    ])
                    .required(),
                ),
            ]),
            JsonSchema::Array(Box::new(JsonSchema::Number)),
            JsonSchema::Tuple {
                prefix_items: vec![],
                items: None,
            },
            JsonSchema::Tuple {
                prefix_items: vec![
                    JsonSchema::Array(Box::new(JsonSchema::Number)),
                    JsonSchema::object(vec![
                        ("foo".into(), JsonSchema::String.optional()),
                        ("bar".into(), JsonSchema::Number.required()),
                        ("baz".into(), JsonSchema::Number.required()),
                    ]),
                ],
                items: None,
            },
            JsonSchema::Tuple {
                prefix_items: vec![
                    JsonSchema::Array(Box::new(JsonSchema::Number)),
                    JsonSchema::object(vec![
                        ("foo".into(), JsonSchema::String.optional()),
                        ("bar".into(), JsonSchema::Number.required()),
                        ("baz".into(), JsonSchema::Number.required()),
                    ]),
                ],
                items: Some(Box::new(JsonSchema::object(vec![
                    ("foo".into(), JsonSchema::String.optional()),
                    ("bar".into(), JsonSchema::Number.required()),
                    ("baz".into(), JsonSchema::Number.required()),
                ]))),
            },
            JsonSchema::Ref("abc".into()),
            JsonSchema::OpenApiResponseRef("def".into()),
            JsonSchema::any_of(vec![]),
            JsonSchema::any_of(vec![JsonSchema::String, JsonSchema::Number]),
            JsonSchema::any_of(vec![
                JsonSchema::Const(Json::Bool(false)),
                JsonSchema::Const(Json::Bool(true)),
            ]),
            JsonSchema::all_of(vec![]),
            JsonSchema::all_of(vec![JsonSchema::String, JsonSchema::Number]),
            JsonSchema::Const(Json::String("abc".into())),
            JsonSchema::Const(Json::parse_int(123)),
        ];
        for schema in schemas {
            let json = schema.clone().to_json();
            let str = serde_json::to_string_pretty(&json.to_serde()).unwrap();
            let from_str = serde_json::from_str::<serde_json::Value>(&str).unwrap();
            let from_serde = Json::from_serde(&from_str);
            let from_json = JsonSchema::from_json(&from_serde).unwrap();
            assert_eq!(schema, from_json,);
        }
    }
}
