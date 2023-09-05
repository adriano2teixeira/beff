
Object.defineProperty(exports, "__esModule", {
  value: true
});
    
const { validators, add_path_to_errors, registerStringFormat, isCustomFormatInvalid } = require('./validators.js').default;

class CoercionFailure {
  constructor(original) {
    this.__isCoercionFailure = true;
    this.original = original
  }
}
function coerce_string(input) {
  return input;
}
const isNumeric = (num) =>
  (typeof num === "number" || (typeof num === "string" && num.trim() !== "")) &&
  !isNaN(num );
function coerce_number(input) {
  if (isNumeric(input)) {
    return Number(input);
  }
  return new CoercionFailure(input);
}
function coerce_boolean(input) {
  if (input === "true" || input === "false") {
    return input === "true";
  }
  if (input === "1" || input === "0") {
    return input === "1";
  }
  return new CoercionFailure(input);
}
function coerce_union(input, ...cases) {
  for (const c of cases) {
    const r = coerce(c, input);
    if (!(r instanceof CoercionFailure)) {
      return r;
    }
  }
  return new CoercionFailure(input);
}
function coerce(coercer, value) {
  return coercer(value);
}

const meta = [
    {
        "method_kind": "use",
        "params": [],
        "pattern": "*",
        "return_validator": function(input) {
            let error_acc_0 = [];
            return error_acc_0;
        }
    },
    {
        "method_kind": "use",
        "params": [],
        "pattern": "/posts/*",
        "return_validator": function(input) {
            let error_acc_0 = [];
            return error_acc_0;
        }
    },
    {
        "method_kind": "get",
        "params": [],
        "pattern": "/",
        "return_validator": function(input) {
            let error_acc_0 = [];
            if (typeof input == "object" && input != null) {
                if (typeof input["message"] != "string") {
                    error_acc_0.push({
                        "error_kind": "NotTypeof",
                        "expected_type": "string",
                        "path": [
                            "responseBody",
                            "message"
                        ],
                        "received": input["message"]
                    });
                }
            } else {
                error_acc_0.push({
                    "error_kind": "NotAnObject",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            return error_acc_0;
        }
    },
    {
        "method_kind": "get",
        "params": [
            {
                "type": "context"
            }
        ],
        "pattern": "/posts",
        "return_validator": function(input) {
            let error_acc_0 = [];
            if (typeof input == "object" && input != null) {
                if (typeof input["ok"] != "boolean") {
                    error_acc_0.push({
                        "error_kind": "NotTypeof",
                        "expected_type": "boolean",
                        "path": [
                            "responseBody",
                            "ok"
                        ],
                        "received": input["ok"]
                    });
                }
                if (Array.isArray(input["posts"])) {
                    for(let index = 0; index < input["posts"].length; index++){
                        const array_item_1 = input["posts"][index];
                        error_acc_0.push(...add_path_to_errors(validators.Post(array_item_1), [
                            "responseBody",
                            "posts",
                            "[" + index + "]"
                        ]));
                    }
                } else {
                    error_acc_0.push({
                        "error_kind": "NotAnArray",
                        "path": [
                            "responseBody",
                            "posts"
                        ],
                        "received": input["posts"]
                    });
                }
            } else {
                error_acc_0.push({
                    "error_kind": "NotAnObject",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            return error_acc_0;
        }
    },
    {
        "method_kind": "post",
        "params": [
            {
                "type": "context"
            },
            {
                "name": "param",
                "required": true,
                "type": "body",
                "validator": function(input) {
                    let error_acc_0 = [];
                    error_acc_0.push(...add_path_to_errors(validators.Param(input), [
                        "requestBody"
                    ]));
                    return error_acc_0;
                }
            }
        ],
        "pattern": "/posts",
        "return_validator": function(input) {
            let error_acc_0 = [];
            let is_ok_1 = false;
            let error_acc_2 = [];
            if (typeof input == "object" && input != null) {
                if (typeof input["error"] != "string") {
                    error_acc_2.push({
                        "error_kind": "NotTypeof",
                        "expected_type": "string",
                        "path": [
                            "responseBody",
                            "error"
                        ],
                        "received": input["error"]
                    });
                }
                if (input["ok"] != false) {
                    error_acc_2.push({
                        "error_kind": "NotEq",
                        "expected_value": false,
                        "path": [
                            "responseBody",
                            "ok"
                        ],
                        "received": input["ok"]
                    });
                }
            } else {
                error_acc_2.push({
                    "error_kind": "NotAnObject",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            is_ok_1 = is_ok_1 || error_acc_2.length === 0;
            let error_acc_3 = [];
            if (typeof input == "object" && input != null) {
                if (input["ok"] != true) {
                    error_acc_3.push({
                        "error_kind": "NotEq",
                        "expected_value": true,
                        "path": [
                            "responseBody",
                            "ok"
                        ],
                        "received": input["ok"]
                    });
                }
                error_acc_3.push(...add_path_to_errors(validators.Post(input["post"]), [
                    "responseBody",
                    "post"
                ]));
            } else {
                error_acc_3.push({
                    "error_kind": "NotAnObject",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            is_ok_1 = is_ok_1 || error_acc_3.length === 0;
            if (!(is_ok_1)) {
                error_acc_0.push({
                    "error_kind": "InvalidUnion",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            return error_acc_0;
        }
    },
    {
        "method_kind": "get",
        "params": [
            {
                "type": "context"
            },
            {
                "coercer": function(input) {
                    return coerce_string(input);
                },
                "name": "id",
                "required": true,
                "type": "path",
                "validator": function(input) {
                    let error_acc_0 = [];
                    if (typeof input != "string") {
                        error_acc_0.push({
                            "error_kind": "NotTypeof",
                            "expected_type": "string",
                            "path": [
                                "id"
                            ],
                            "received": input
                        });
                    }
                    return error_acc_0;
                }
            }
        ],
        "pattern": "/posts/{id}",
        "return_validator": function(input) {
            let error_acc_0 = [];
            let is_ok_1 = false;
            let error_acc_2 = [];
            if (typeof input == "object" && input != null) {
                if (typeof input["error"] != "string") {
                    error_acc_2.push({
                        "error_kind": "NotTypeof",
                        "expected_type": "string",
                        "path": [
                            "responseBody",
                            "error"
                        ],
                        "received": input["error"]
                    });
                }
                if (input["ok"] != false) {
                    error_acc_2.push({
                        "error_kind": "NotEq",
                        "expected_value": false,
                        "path": [
                            "responseBody",
                            "ok"
                        ],
                        "received": input["ok"]
                    });
                }
            } else {
                error_acc_2.push({
                    "error_kind": "NotAnObject",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            is_ok_1 = is_ok_1 || error_acc_2.length === 0;
            let error_acc_3 = [];
            if (typeof input == "object" && input != null) {
                if (input["ok"] != true) {
                    error_acc_3.push({
                        "error_kind": "NotEq",
                        "expected_value": true,
                        "path": [
                            "responseBody",
                            "ok"
                        ],
                        "received": input["ok"]
                    });
                }
                error_acc_3.push(...add_path_to_errors(validators.Post(input["post"]), [
                    "responseBody",
                    "post"
                ]));
            } else {
                error_acc_3.push({
                    "error_kind": "NotAnObject",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            is_ok_1 = is_ok_1 || error_acc_3.length === 0;
            if (!(is_ok_1)) {
                error_acc_0.push({
                    "error_kind": "InvalidUnion",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            return error_acc_0;
        }
    },
    {
        "method_kind": "put",
        "params": [
            {
                "type": "context"
            },
            {
                "coercer": function(input) {
                    return coerce_string(input);
                },
                "name": "id",
                "required": true,
                "type": "path",
                "validator": function(input) {
                    let error_acc_0 = [];
                    if (typeof input != "string") {
                        error_acc_0.push({
                            "error_kind": "NotTypeof",
                            "expected_type": "string",
                            "path": [
                                "id"
                            ],
                            "received": input
                        });
                    }
                    return error_acc_0;
                }
            },
            {
                "name": "param",
                "required": true,
                "type": "body",
                "validator": function(input) {
                    let error_acc_0 = [];
                    error_acc_0.push(...add_path_to_errors(validators.Param(input), [
                        "requestBody"
                    ]));
                    return error_acc_0;
                }
            }
        ],
        "pattern": "/posts/{id}",
        "return_validator": function(input) {
            let error_acc_0 = [];
            if (typeof input == "object" && input != null) {
                if (typeof input["ok"] != "boolean") {
                    error_acc_0.push({
                        "error_kind": "NotTypeof",
                        "expected_type": "boolean",
                        "path": [
                            "responseBody",
                            "ok"
                        ],
                        "received": input["ok"]
                    });
                }
            } else {
                error_acc_0.push({
                    "error_kind": "NotAnObject",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            return error_acc_0;
        }
    },
    {
        "method_kind": "delete",
        "params": [
            {
                "type": "context"
            },
            {
                "coercer": function(input) {
                    return coerce_string(input);
                },
                "name": "id",
                "required": true,
                "type": "path",
                "validator": function(input) {
                    let error_acc_0 = [];
                    if (typeof input != "string") {
                        error_acc_0.push({
                            "error_kind": "NotTypeof",
                            "expected_type": "string",
                            "path": [
                                "id"
                            ],
                            "received": input
                        });
                    }
                    return error_acc_0;
                }
            }
        ],
        "pattern": "/posts/{id}",
        "return_validator": function(input) {
            let error_acc_0 = [];
            if (typeof input == "object" && input != null) {
                if (typeof input["ok"] != "boolean") {
                    error_acc_0.push({
                        "error_kind": "NotTypeof",
                        "expected_type": "boolean",
                        "path": [
                            "responseBody",
                            "ok"
                        ],
                        "received": input["ok"]
                    });
                }
            } else {
                error_acc_0.push({
                    "error_kind": "NotAnObject",
                    "path": [
                        "responseBody"
                    ],
                    "received": input
                });
            }
            return error_acc_0;
        }
    }
];

const schema =  {
  "components": {
    "responses": {
      "DecodeError": {
        "content": {
          "application/json": {
            "schema": {
              "properties": {
                "message": {
                  "type": "string"
                }
              },
              "required": [
                "message"
              ],
              "type": "object"
            }
          }
        },
        "description": "Invalid parameters or request body"
      },
      "UnexpectedError": {
        "content": {
          "application/json": {
            "schema": {
              "properties": {
                "message": {
                  "type": "string"
                }
              },
              "required": [
                "message"
              ],
              "type": "object"
            }
          }
        },
        "description": "Unexpected Error"
      }
    },
    "schemas": {
      "Param": {
        "properties": {
          "body": {
            "type": "string"
          },
          "title": {
            "type": "string"
          }
        },
        "required": [
          "body",
          "title"
        ],
        "type": "object"
      },
      "Post": {
        "properties": {
          "body": {
            "type": "string"
          },
          "id": {
            "type": "string"
          },
          "title": {
            "type": "string"
          }
        },
        "required": [
          "body",
          "id",
          "title"
        ],
        "type": "object"
      }
    }
  },
  "info": {
    "title": "No title",
    "version": "0.0.0"
  },
  "openapi": "3.1.0",
  "paths": {
    "/": {
      "get": {
        "parameters": [],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "properties": {
                    "message": {
                      "type": "string"
                    }
                  },
                  "required": [
                    "message"
                  ],
                  "type": "object"
                }
              }
            },
            "description": "Successful Operation"
          },
          "422": {
            "$ref": "#/components/responses/DecodeError"
          },
          "500": {
            "$ref": "#/components/responses/UnexpectedError"
          }
        }
      }
    },
    "/posts": {
      "get": {
        "parameters": [],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "properties": {
                    "ok": {
                      "type": "boolean"
                    },
                    "posts": {
                      "items": {
                        "$ref": "#/components/schemas/Post"
                      },
                      "type": "array"
                    }
                  },
                  "required": [
                    "ok",
                    "posts"
                  ],
                  "type": "object"
                }
              }
            },
            "description": "Successful Operation"
          },
          "422": {
            "$ref": "#/components/responses/DecodeError"
          },
          "500": {
            "$ref": "#/components/responses/UnexpectedError"
          }
        }
      },
      "post": {
        "parameters": [],
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/Param"
              }
            }
          },
          "required": true
        },
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "anyOf": [
                    {
                      "properties": {
                        "error": {
                          "type": "string"
                        },
                        "ok": {
                          "const": false
                        }
                      },
                      "required": [
                        "error",
                        "ok"
                      ],
                      "type": "object"
                    },
                    {
                      "properties": {
                        "ok": {
                          "const": true
                        },
                        "post": {
                          "$ref": "#/components/schemas/Post"
                        }
                      },
                      "required": [
                        "ok",
                        "post"
                      ],
                      "type": "object"
                    }
                  ]
                }
              }
            },
            "description": "Successful Operation"
          },
          "422": {
            "$ref": "#/components/responses/DecodeError"
          },
          "500": {
            "$ref": "#/components/responses/UnexpectedError"
          }
        }
      }
    },
    "/posts/{id}": {
      "delete": {
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "properties": {
                    "ok": {
                      "type": "boolean"
                    }
                  },
                  "required": [
                    "ok"
                  ],
                  "type": "object"
                }
              }
            },
            "description": "Successful Operation"
          },
          "422": {
            "$ref": "#/components/responses/DecodeError"
          },
          "500": {
            "$ref": "#/components/responses/UnexpectedError"
          }
        }
      },
      "get": {
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "anyOf": [
                    {
                      "properties": {
                        "error": {
                          "type": "string"
                        },
                        "ok": {
                          "const": false
                        }
                      },
                      "required": [
                        "error",
                        "ok"
                      ],
                      "type": "object"
                    },
                    {
                      "properties": {
                        "ok": {
                          "const": true
                        },
                        "post": {
                          "$ref": "#/components/schemas/Post"
                        }
                      },
                      "required": [
                        "ok",
                        "post"
                      ],
                      "type": "object"
                    }
                  ]
                }
              }
            },
            "description": "Successful Operation"
          },
          "422": {
            "$ref": "#/components/responses/DecodeError"
          },
          "500": {
            "$ref": "#/components/responses/UnexpectedError"
          }
        }
      },
      "put": {
        "parameters": [
          {
            "in": "path",
            "name": "id",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/Param"
              }
            }
          },
          "required": true
        },
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "properties": {
                    "ok": {
                      "type": "boolean"
                    }
                  },
                  "required": [
                    "ok"
                  ],
                  "type": "object"
                }
              }
            },
            "description": "Successful Operation"
          },
          "422": {
            "$ref": "#/components/responses/DecodeError"
          },
          "500": {
            "$ref": "#/components/responses/UnexpectedError"
          }
        }
      }
    }
  }
} ;
exports.default = { meta, schema };