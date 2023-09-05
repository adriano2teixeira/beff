
import vals from "./validators.js"; const { validators, add_path_to_errors, registerStringFormat, isCustomFormatInvalid } = vals;

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
        "method_kind": "get",
        "params": [
            {
                "type": "context"
            },
            {
                "coercer": function(input) {
                    return coerce_string(input);
                },
                "name": "name",
                "required": true,
                "type": "path",
                "validator": function(input) {
                    let error_acc_0 = [];
                    if (typeof input != "string") {
                        error_acc_0.push({
                            "error_kind": "NotTypeof",
                            "expected_type": "string",
                            "path": [
                                "name"
                            ],
                            "received": input
                        });
                    }
                    return error_acc_0;
                }
            }
        ],
        "pattern": "/{name}",
        "return_validator": function(input) {
            let error_acc_0 = [];
            error_acc_0.push(...add_path_to_errors(validators.User(input), [
                "responseBody"
            ]));
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
                "name": "uuid",
                "required": true,
                "type": "path",
                "validator": function(input) {
                    let error_acc_0 = [];
                    if (typeof input != "string") {
                        error_acc_0.push({
                            "error_kind": "NotTypeof",
                            "expected_type": "string",
                            "path": [
                                "uuid"
                            ],
                            "received": input
                        });
                    }
                    return error_acc_0;
                }
            },
            {
                "coercer": function(input) {
                    return coerce_string(input);
                },
                "name": "p",
                "required": true,
                "type": "query",
                "validator": function(input) {
                    let error_acc_0 = [];
                    error_acc_0.push(...add_path_to_errors(validators.Password(input), [
                        "p"
                    ]));
                    return error_acc_0;
                }
            }
        ],
        "pattern": "/check-uuid/{uuid}",
        "return_validator": function(input) {
            let error_acc_0 = [];
            error_acc_0.push(...add_path_to_errors(validators.StartsWithA(input), [
                "responseBody"
            ]));
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
      "Password": {
        "format": "password",
        "type": "string"
      },
      "StartsWithA": {
        "format": "StartsWithA",
        "type": "string"
      },
      "User": {
        "properties": {
          "age": {
            "type": "number"
          },
          "name": {
            "type": "string"
          }
        },
        "required": [
          "age",
          "name"
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
    "/check-uuid/{uuid}": {
      "get": {
        "parameters": [
          {
            "in": "path",
            "name": "uuid",
            "required": true,
            "schema": {
              "type": "string"
            }
          },
          {
            "in": "query",
            "name": "p",
            "required": true,
            "schema": {
              "$ref": "#/components/schemas/Password"
            }
          }
        ],
        "responses": {
          "200": {
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/StartsWithA"
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
    "/{name}": {
      "get": {
        "parameters": [
          {
            "in": "path",
            "name": "name",
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
                  "$ref": "#/components/schemas/User"
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
export default { meta, schema };