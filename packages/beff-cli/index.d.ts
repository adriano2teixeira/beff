export type Header<T> = T;
export type Cookie<T> = T;
export type StringFormat<Tag extends string> = string & { __customType: Tag };

export * as Formats from "./formats";

// TODO: validator and coercer are server specific, move these types to server, duplicate in client

export type MetaParamClient = {
  type: "path" | "query" | "cookie" | "header" | "body" | "context";
  name: string;
  required: boolean;
};

export type MetaParamServer = MetaParamClient & {
  validator: any;
  coercer: any;
};
export type HandlerMetaClient = {
  method_kind: "get" | "post" | "put" | "delete" | "patch" | "options" | "use";
  params: MetaParamClient[];
  pattern: string;
};
export type HandlerMetaServer = HandlerMetaClient & {
  return_validator: any;
  params: MetaParamServer[];
};

export type ErrorVariant<T> = {
  error_kind: T;
  path: string[];
  received: unknown;
};
export type DecodeError =
  | ErrorVariant<"NotAnObject">
  | ErrorVariant<"NotAnArray">
  | ErrorVariant<"InvalidUnion">
  | (ErrorVariant<"StringFormatCheckFailed"> & {
      expected_type: string;
    })
  | (ErrorVariant<"NotTypeof"> & {
      expected_type: string;
    })
  | (ErrorVariant<"NotEq"> & {
      expected_value: unknown;
    });

// TODO: fix me
export type OpenApiServer = any;
