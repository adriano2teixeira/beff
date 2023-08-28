import { HTTPException } from "hono/http-exception";
import { getCookie } from "hono/cookie";
function add_path_to_errors(errors, path) {
    return errors.map((e) => ({ ...e, path: [...path, ...e.path] }));
}
function coerce_string(input) {
    return input;
}
class CoercionFailure {
}
const isNumeric = (num) => (typeof num === "number" || (typeof num === "string" && num.trim() !== "")) &&
    !isNaN(num);
function coerce_number(input) {
    if (isNumeric(input)) {
        return Number(input);
    }
    return new CoercionFailure();
}
function coerce_boolean(input) {
    if (input === "true" || input === "false") {
        return input === "true";
    }
    if (input === "1" || input === "0") {
        return input === "1";
    }
    return new CoercionFailure();
}
function coerce_bigint(_input) {
    throw new Error("not implemented");
    // return Object(input);
}
function coerce_union(_input, ..._cases) {
    throw new Error("not implemented");
    // return Object(input);
}
function coerce_intersection(_input, ..._cases) {
    throw new Error("not implemented");
    // return Object(input);
}
const template = `
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta
      name="description"
      content="SwaggerUI"
    />
    <title>SwaggerUI</title>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/normalize/8.0.1/normalize.min.css" />
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.0.0/swagger-ui.css" />

  </head>
  <body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5.0.0/swagger-ui-bundle.js" crossorigin></script>
  <script src="https://unpkg.com/swagger-ui-dist@5.0.0/swagger-ui-standalone-preset.js" crossorigin></script>
  <script>
    window.onload = () => {
      window.ui = SwaggerUIBundle({
        url: '/api/v3/openapi.json',
        dom_id: '#swagger-ui',
        presets: [
          SwaggerUIBundle.presets.apis,
          SwaggerUIStandalonePreset
        ],
        layout: "StandaloneLayout",
      });
    };
  </script>
  </body>
</html>
`;
const registerDocs = (app, metadata, servers) => {
    app.get("/v3/openapi.json", (req) => req.json({
        ...metadata["schema"],
        servers,
    }));
    app.get("/docs", (c) => c.html(template));
};
const printKind = (kind) => {
    switch (kind[0]) {
        case "NotTypeof": {
            return `expected ${kind[1]}`;
        }
        default: {
            return "unknown";
        }
    }
};
const printValidationErrors = (errors) => {
    return errors
        .map((e) => {
        return `Decoder error at ${e.path.join(".")}: ${printKind(e.kind)}.`;
    })
        .join("\n");
};
const decodeWithMessage = (validator, value) => {
    const errs = validator(value);
    if (errs.length > 0) {
        throw new HTTPException(422, { message: printValidationErrors(errs) });
    }
    return value;
};
const decodeNoMessage = (validator, value) => {
    const errs = validator(value);
    if (errs.length > 0) {
        throw new HTTPException(422, { message: "Internal validation error" });
    }
    return value;
};
const coerce = (coercer, value) => {
    return coercer(value);
};
const handleMethod = async (c, meta, handler) => {
    const resolverParamsPromise = meta.params.map(async (p) => {
        switch (p.type) {
            case "path": {
                const value = c.req.param(p.name);
                const coerced = coerce(p.coercer, value);
                return decodeWithMessage(p.validator, coerced);
            }
            case "query": {
                const value = c.req.query(p.name);
                const coerced = coerce(p.coercer, value);
                return decodeWithMessage(p.validator, coerced);
            }
            case "cookie": {
                const value = getCookie(c, p.name);
                const coerced = coerce(p.coercer, value);
                return decodeWithMessage(p.validator, coerced);
            }
            case "header": {
                const value = c.req.header(p.name);
                const coerced = coerce(p.coercer, value);
                return decodeWithMessage(p.validator, coerced);
            }
            case "body": {
                const value = await c.req.json();
                return decodeWithMessage(p.validator, value);
            }
        }
        throw new Error("not implemented: " + p.type);
    });
    const resolverParams = await Promise.all(resolverParamsPromise);
    const result = await handler(...resolverParams);
    return c.json(decodeNoMessage(meta.return_validator, result));
};
const toHonoPattern = (pattern) => {
    // replace {id} with :id
    return pattern.replace(/\{(\w+)\}/g, ":$1");
};
export function registerRouter(options) {
    registerDocs(options.app, meta, options.openApi?.servers ?? []);
    const handlersMeta = meta["handlersMeta"];
    for (const meta of handlersMeta) {
        const key = `${meta.method_kind.toUpperCase()}${meta.pattern}`;
        const handlerFunction = options.router[key];
        if (handlerFunction == null) {
            throw new Error("handler not found: " + key);
        }
        const app = options.app;
        switch (meta.method_kind) {
            case "get":
            case "post":
            case "put":
            case "delete":
            case "patch":
            case "options": {
                app[meta.method_kind](toHonoPattern(meta.pattern), async (c) => handleMethod(c, meta, handlerFunction));
                break;
            }
            default: {
                throw new Error("Method not recognized: " + meta.method_kind);
            }
        }
    }
}
export const todo = () => {
    throw new Error("TODO: not implemented");
};

function validate_DataTypesKitchenSink(input) {
    let error_acc_0 = [];
    if (typeof input == "object" && input != null) {
        if (typeof input["basic"] == "object" && input["basic"] != null) {
            if (typeof input["basic"]["a"] != "string") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "string"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "basic",
                        "a"
                    ]
                });
            }
            if (typeof input["basic"]["b"] != "number") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "number"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "basic",
                        "b"
                    ]
                });
            }
            if (typeof input["basic"]["c"] != "boolean") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "boolean"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "basic",
                        "c"
                    ]
                });
            }
            if (typeof input["basic"]["d"] != "bigint") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "bigint"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "basic",
                        "d"
                    ]
                });
            }
        } else {
            error_acc_0.push({
                "kind": [
                    "NotAnObject"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "basic"
                ]
            });
        }
        if (Array.isArray(input["array1"])) {
            for (const array_item_1 of input["array1"]){
                if (typeof array_item_1 != "string") {
                    error_acc_0.push({
                        "kind": [
                            "NotTypeof",
                            "string"
                        ],
                        "path": [
                            "DataTypesKitchenSink",
                            "array1",
                            "[]"
                        ]
                    });
                }
            }
        } else {
            error_acc_0.push({
                "kind": [
                    "NotAnArray"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "array1"
                ]
            });
        }
        if (Array.isArray(input["array2"])) {
            for (const array_item_2 of input["array2"]){
                if (typeof array_item_2 != "string") {
                    error_acc_0.push({
                        "kind": [
                            "NotTypeof",
                            "string"
                        ],
                        "path": [
                            "DataTypesKitchenSink",
                            "array2",
                            "[]"
                        ]
                    });
                }
            }
        } else {
            error_acc_0.push({
                "kind": [
                    "NotAnArray"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "array2"
                ]
            });
        }
        if (Array.isArray(input["tuple1"])) {
            if (typeof input["tuple1"][0] != "string") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "string"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "tuple1",
                        "[0]"
                    ]
                });
            }
        } else {
            error_acc_0.push({
                "kind": [
                    "NotAnArray"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "tuple1"
                ]
            });
        }
        if (Array.isArray(input["tuple2"])) {
            if (typeof input["tuple2"][0] != "string") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "string"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "tuple2",
                        "[0]"
                    ]
                });
            }
            if (typeof input["tuple2"][1] != "string") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "string"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "tuple2",
                        "[1]"
                    ]
                });
            }
        } else {
            error_acc_0.push({
                "kind": [
                    "NotAnArray"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "tuple2"
                ]
            });
        }
        if (Array.isArray(input["tuple_rest"])) {
            if (typeof input["tuple_rest"][0] != "string") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "string"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "tuple_rest",
                        "[0]"
                    ]
                });
            }
            if (typeof input["tuple_rest"][1] != "string") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "string"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "tuple_rest",
                        "[1]"
                    ]
                });
            }
            if (Array.isArray(input["tuple_rest"].slice(2))) {
                for (const array_item_3 of input["tuple_rest"].slice(2)){
                    if (typeof array_item_3 != "number") {
                        error_acc_0.push({
                            "kind": [
                                "NotTypeof",
                                "number"
                            ],
                            "path": [
                                "DataTypesKitchenSink",
                                "tuple_rest",
                                "[]",
                                "[]"
                            ]
                        });
                    }
                }
            } else {
                error_acc_0.push({
                    "kind": [
                        "NotAnArray"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "tuple_rest",
                        "[]"
                    ]
                });
            }
        } else {
            error_acc_0.push({
                "kind": [
                    "NotAnArray"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "tuple_rest"
                ]
            });
        }
        let is_ok_4 = false;
        let error_acc_5 = [];
        if (typeof input["nullable"] != "string") {
            error_acc_5.push({
                "kind": [
                    "NotTypeof",
                    "string"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "nullable"
                ]
            });
        }
        is_ok_4 = is_ok_4 || error_acc_5.length === 0;
        let error_acc_6 = [];
        if (input["nullable"] != null) {
            error_acc_6.push({
                "kind": [
                    "NotEq",
                    null
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "nullable"
                ]
            });
        }
        is_ok_4 = is_ok_4 || error_acc_6.length === 0;
        if (!(is_ok_4)) {
            error_acc_0.push({
                "kind": [
                    "InvalidUnion"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "nullable"
                ]
            });
        }
        let is_ok_7 = false;
        let error_acc_8 = [];
        if (typeof input["many_nullable"] != "number") {
            error_acc_8.push({
                "kind": [
                    "NotTypeof",
                    "number"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "many_nullable"
                ]
            });
        }
        is_ok_7 = is_ok_7 || error_acc_8.length === 0;
        let error_acc_9 = [];
        if (typeof input["many_nullable"] != "string") {
            error_acc_9.push({
                "kind": [
                    "NotTypeof",
                    "string"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "many_nullable"
                ]
            });
        }
        is_ok_7 = is_ok_7 || error_acc_9.length === 0;
        let error_acc_10 = [];
        if (input["many_nullable"] != null) {
            error_acc_10.push({
                "kind": [
                    "NotEq",
                    null
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "many_nullable"
                ]
            });
        }
        is_ok_7 = is_ok_7 || error_acc_10.length === 0;
        if (!(is_ok_7)) {
            error_acc_0.push({
                "kind": [
                    "InvalidUnion"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "many_nullable"
                ]
            });
        }
        if (input["optional_prop"] != null) {
            if (typeof input["optional_prop"] != "string") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "string"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "optional_prop"
                    ]
                });
            }
        }
        let is_ok_11 = false;
        let error_acc_12 = [];
        if (typeof input["union_with_undefined"] != "string") {
            error_acc_12.push({
                "kind": [
                    "NotTypeof",
                    "string"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "union_with_undefined"
                ]
            });
        }
        is_ok_11 = is_ok_11 || error_acc_12.length === 0;
        let error_acc_13 = [];
        if (input["union_with_undefined"] != null) {
            error_acc_13.push({
                "kind": [
                    "NotEq",
                    null
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "union_with_undefined"
                ]
            });
        }
        is_ok_11 = is_ok_11 || error_acc_13.length === 0;
        if (!(is_ok_11)) {
            error_acc_0.push({
                "kind": [
                    "InvalidUnion"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "union_with_undefined"
                ]
            });
        }
        let is_ok_14 = false;
        let error_acc_15 = [];
        if (typeof input["union_of_many"] != "string") {
            error_acc_15.push({
                "kind": [
                    "NotTypeof",
                    "string"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "union_of_many"
                ]
            });
        }
        is_ok_14 = is_ok_14 || error_acc_15.length === 0;
        let error_acc_16 = [];
        if (typeof input["union_of_many"] != "number") {
            error_acc_16.push({
                "kind": [
                    "NotTypeof",
                    "number"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "union_of_many"
                ]
            });
        }
        is_ok_14 = is_ok_14 || error_acc_16.length === 0;
        let error_acc_17 = [];
        if (typeof input["union_of_many"] != "boolean") {
            error_acc_17.push({
                "kind": [
                    "NotTypeof",
                    "boolean"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "union_of_many"
                ]
            });
        }
        is_ok_14 = is_ok_14 || error_acc_17.length === 0;
        if (!(is_ok_14)) {
            error_acc_0.push({
                "kind": [
                    "InvalidUnion"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "union_of_many"
                ]
            });
        }
        if (typeof input["literals"] == "object" && input["literals"] != null) {
            if (input["literals"]["a"] != "a") {
                error_acc_0.push({
                    "kind": [
                        "NotEq",
                        "a"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "literals",
                        "a"
                    ]
                });
            }
            if (input["literals"]["b"] != 1) {
                error_acc_0.push({
                    "kind": [
                        "NotEq",
                        1
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "literals",
                        "b"
                    ]
                });
            }
            if (input["literals"]["c"] != true) {
                error_acc_0.push({
                    "kind": [
                        "NotEq",
                        true
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "literals",
                        "c"
                    ]
                });
            }
        } else {
            error_acc_0.push({
                "kind": [
                    "NotAnObject"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "literals"
                ]
            });
        }
        let is_ok_18 = false;
        let error_acc_19 = [];
        if (input["enum"] != "a") {
            error_acc_19.push({
                "kind": [
                    "NotEq",
                    "a"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "enum"
                ]
            });
        }
        is_ok_18 = is_ok_18 || error_acc_19.length === 0;
        let error_acc_20 = [];
        if (input["enum"] != "b") {
            error_acc_20.push({
                "kind": [
                    "NotEq",
                    "b"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "enum"
                ]
            });
        }
        is_ok_18 = is_ok_18 || error_acc_20.length === 0;
        let error_acc_21 = [];
        if (input["enum"] != "c") {
            error_acc_21.push({
                "kind": [
                    "NotEq",
                    "c"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "enum"
                ]
            });
        }
        is_ok_18 = is_ok_18 || error_acc_21.length === 0;
        if (!(is_ok_18)) {
            error_acc_0.push({
                "kind": [
                    "InvalidUnion"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "enum"
                ]
            });
        }
        if (Array.isArray(input["tuple_lit"])) {
            if (input["tuple_lit"][0] != "a") {
                error_acc_0.push({
                    "kind": [
                        "NotEq",
                        "a"
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "tuple_lit",
                        "[0]"
                    ]
                });
            }
            if (input["tuple_lit"][1] != 1) {
                error_acc_0.push({
                    "kind": [
                        "NotEq",
                        1
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "tuple_lit",
                        "[1]"
                    ]
                });
            }
            if (input["tuple_lit"][2] != true) {
                error_acc_0.push({
                    "kind": [
                        "NotEq",
                        true
                    ],
                    "path": [
                        "DataTypesKitchenSink",
                        "tuple_lit",
                        "[2]"
                    ]
                });
            }
        } else {
            error_acc_0.push({
                "kind": [
                    "NotAnArray"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "tuple_lit"
                ]
            });
        }
        if (input["str_template"] != "ab") {
            error_acc_0.push({
                "kind": [
                    "NotEq",
                    "ab"
                ],
                "path": [
                    "DataTypesKitchenSink",
                    "str_template"
                ]
            });
        }
    } else {
        error_acc_0.push({
            "kind": [
                "NotAnObject"
            ],
            "path": [
                "DataTypesKitchenSink"
            ]
        });
    }
    return error_acc_0;
}
function validate_User(input) {
    let error_acc_0 = [];
    if (typeof input == "object" && input != null) {
        if (typeof input["id"] != "number") {
            error_acc_0.push({
                "kind": [
                    "NotTypeof",
                    "number"
                ],
                "path": [
                    "User",
                    "id"
                ]
            });
        }
        if (typeof input["name"] != "string") {
            error_acc_0.push({
                "kind": [
                    "NotTypeof",
                    "string"
                ],
                "path": [
                    "User",
                    "name"
                ]
            });
        }
        if (Array.isArray(input["entities"])) {
            for (const array_item_1 of input["entities"]){
                error_acc_0.push(...add_path_to_errors(validate_UserEntity(array_item_1), [
                    "User",
                    "entities",
                    "[]"
                ]));
            }
        } else {
            error_acc_0.push({
                "kind": [
                    "NotAnArray"
                ],
                "path": [
                    "User",
                    "entities"
                ]
            });
        }
        if (input["optional_prop"] != null) {
            if (typeof input["optional_prop"] != "string") {
                error_acc_0.push({
                    "kind": [
                        "NotTypeof",
                        "string"
                    ],
                    "path": [
                        "User",
                        "optional_prop"
                    ]
                });
            }
        }
    } else {
        error_acc_0.push({
            "kind": [
                "NotAnObject"
            ],
            "path": [
                "User"
            ]
        });
    }
    return error_acc_0;
}
function validate_UserEntity(input) {
    let error_acc_0 = [];
    if (typeof input == "object" && input != null) {
        if (typeof input["id"] != "string") {
            error_acc_0.push({
                "kind": [
                    "NotTypeof",
                    "string"
                ],
                "path": [
                    "UserEntity",
                    "id"
                ]
            });
        }
    } else {
        error_acc_0.push({
            "kind": [
                "NotAnObject"
            ],
            "path": [
                "UserEntity"
            ]
        });
    }
    return error_acc_0;
}
export const meta = {
    "handlersMeta": [
        {
            "method_kind": "get",
            "params": [],
            "pattern": "/data-types-kitchen-sink",
            "return_validator": function(input) {
                let error_acc_0 = [];
                error_acc_0.push(...add_path_to_errors(validate_DataTypesKitchenSink(input), [
                    "[GET] /data-types-kitchen-sink.response_body"
                ]));
                return error_acc_0;
            }
        },
        {
            "method_kind": "get",
            "params": [],
            "pattern": "/anon-func",
            "return_validator": function(input) {
                let error_acc_0 = [];
                if (typeof input != "string") {
                    error_acc_0.push({
                        "kind": [
                            "NotTypeof",
                            "string"
                        ],
                        "path": [
                            "[GET] /anon-func.response_body"
                        ]
                    });
                }
                return error_acc_0;
            }
        },
        {
            "method_kind": "get",
            "params": [
                {
                    "type": "header",
                    "name": "user_agent",
                    "required": true,
                    "validator": function(input) {
                        let error_acc_0 = [];
                        if (typeof input != "string") {
                            error_acc_0.push({
                                "kind": [
                                    "NotTypeof",
                                    "string"
                                ],
                                "path": [
                                    'Header Argument "user_agent"'
                                ]
                            });
                        }
                        return error_acc_0;
                    },
                    "coercer": function(input) {
                        return coerce_string(input);
                    }
                },
                {
                    "type": "cookie",
                    "name": "ads_id",
                    "required": true,
                    "validator": function(input) {
                        let error_acc_0 = [];
                        if (typeof input != "string") {
                            error_acc_0.push({
                                "kind": [
                                    "NotTypeof",
                                    "string"
                                ],
                                "path": [
                                    'Cookie Argument "ads_id"'
                                ]
                            });
                        }
                        return error_acc_0;
                    },
                    "coercer": function(input) {
                        return coerce_string(input);
                    }
                }
            ],
            "pattern": "/users",
            "return_validator": function(input) {
                let error_acc_0 = [];
                if (Array.isArray(input)) {
                    for (const array_item_1 of input){
                        if (typeof array_item_1 != "string") {
                            error_acc_0.push({
                                "kind": [
                                    "NotTypeof",
                                    "string"
                                ],
                                "path": [
                                    "[GET] /users.response_body",
                                    "[]"
                                ]
                            });
                        }
                    }
                } else {
                    error_acc_0.push({
                        "kind": [
                            "NotAnArray"
                        ],
                        "path": [
                            "[GET] /users.response_body"
                        ]
                    });
                }
                return error_acc_0;
            }
        },
        {
            "method_kind": "get",
            "params": [
                {
                    "type": "path",
                    "name": "id",
                    "required": true,
                    "validator": function(input) {
                        let error_acc_0 = [];
                        if (typeof input != "number") {
                            error_acc_0.push({
                                "kind": [
                                    "NotTypeof",
                                    "number"
                                ],
                                "path": [
                                    'Path Parameter "id"'
                                ]
                            });
                        }
                        return error_acc_0;
                    },
                    "coercer": function(input) {
                        return coerce_number(input);
                    }
                }
            ],
            "pattern": "/users/{id}",
            "return_validator": function(input) {
                let error_acc_0 = [];
                error_acc_0.push(...add_path_to_errors(validate_User(input), [
                    "[GET] /users/{id}.response_body"
                ]));
                return error_acc_0;
            }
        }
    ],
    "schema": {
        "openapi": "3.1.0",
        "info": {
            "description": "Optional multiline or single-line description in [CommonMark](http://commonmark.org/help/) or HTML.",
            "title": "Sample API",
            "version": "0.1.9"
        },
        "paths": {
            "/data-types-kitchen-sink": {
                "get": {
                    "parameters": [],
                    "responses": {
                        "200": {
                            "description": "successful operation",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/DataTypesKitchenSink"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/anon-func": {
                "get": {
                    "parameters": [],
                    "responses": {
                        "200": {
                            "description": "successful operation",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "string"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/users": {
                "get": {
                    "summary": "Returns a list of users.",
                    "description": "Optional extended description in CommonMark or HTML.",
                    "parameters": [
                        {
                            "name": "user_agent",
                            "in": "header",
                            "required": true,
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "ads_id",
                            "in": "cookie",
                            "required": true,
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "successful operation",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": "string"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/users/{id}": {
                "get": {
                    "summary": "Returns the user.",
                    "description": "Optional extended description in CommonMark or HTML...",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "description": "The user id.",
                            "required": true,
                            "schema": {
                                "type": "number"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "successful operation",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/User"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "DataTypesKitchenSink": {
                    "type": "object",
                    "required": [
                        "basic",
                        "array1",
                        "array2",
                        "tuple1",
                        "tuple2",
                        "tuple_rest",
                        "nullable",
                        "many_nullable",
                        "union_with_undefined",
                        "union_of_many",
                        "literals",
                        "enum",
                        "tuple_lit",
                        "str_template"
                    ],
                    "properties": {
                        "basic": {
                            "type": "object",
                            "required": [
                                "a",
                                "b",
                                "c",
                                "d"
                            ],
                            "properties": {
                                "a": {
                                    "type": "string"
                                },
                                "b": {
                                    "type": "number"
                                },
                                "c": {
                                    "type": "boolean"
                                },
                                "d": {
                                    "type": "integer"
                                }
                            }
                        },
                        "array1": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        },
                        "array2": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        },
                        "tuple1": {
                            "type": "array",
                            "prefixItems": [
                                {
                                    "type": "string"
                                }
                            ],
                            "minItems": 1,
                            "maxItems": 1
                        },
                        "tuple2": {
                            "type": "array",
                            "prefixItems": [
                                {
                                    "type": "string"
                                },
                                {
                                    "type": "string"
                                }
                            ],
                            "minItems": 2,
                            "maxItems": 2
                        },
                        "tuple_rest": {
                            "type": "array",
                            "prefixItems": [
                                {
                                    "type": "string"
                                },
                                {
                                    "type": "string"
                                }
                            ],
                            "items": {
                                "type": "number"
                            }
                        },
                        "nullable": {
                            "anyOf": [
                                {
                                    "type": "string"
                                },
                                {
                                    "type": "null"
                                }
                            ]
                        },
                        "many_nullable": {
                            "anyOf": [
                                {
                                    "type": "number"
                                },
                                {
                                    "type": "string"
                                },
                                {
                                    "type": "null"
                                }
                            ]
                        },
                        "optional_prop": {
                            "type": "string"
                        },
                        "union_with_undefined": {
                            "anyOf": [
                                {
                                    "type": "string"
                                },
                                {
                                    "type": "null"
                                }
                            ]
                        },
                        "union_of_many": {
                            "anyOf": [
                                {
                                    "type": "string"
                                },
                                {
                                    "type": "number"
                                },
                                {
                                    "type": "boolean"
                                }
                            ]
                        },
                        "literals": {
                            "type": "object",
                            "required": [
                                "a",
                                "b",
                                "c"
                            ],
                            "properties": {
                                "a": {
                                    "const": "a"
                                },
                                "b": {
                                    "const": 1
                                },
                                "c": {
                                    "const": true
                                }
                            }
                        },
                        "enum": {
                            "enum": [
                                "a",
                                "b",
                                "c"
                            ]
                        },
                        "tuple_lit": {
                            "type": "array",
                            "prefixItems": [
                                {
                                    "const": "a"
                                },
                                {
                                    "const": 1
                                },
                                {
                                    "const": true
                                }
                            ],
                            "minItems": 3,
                            "maxItems": 3
                        },
                        "str_template": {
                            "const": "ab"
                        }
                    }
                },
                "User": {
                    "type": "object",
                    "required": [
                        "id",
                        "name",
                        "entities"
                    ],
                    "properties": {
                        "id": {
                            "type": "number"
                        },
                        "name": {
                            "type": "string"
                        },
                        "entities": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/UserEntity"
                            }
                        },
                        "optional_prop": {
                            "type": "string"
                        }
                    }
                },
                "UserEntity": {
                    "type": "object",
                    "required": [
                        "id"
                    ],
                    "properties": {
                        "id": {
                            "type": "string"
                        }
                    }
                }
            }
        }
    }
};

