import * as fs from "fs";
import * as path from "path";
import { Bundler, WritableModules } from "./bundler";
import { ProjectJson, ProjectModule } from "./project";

const RUNTIME_DTS = `
import {JSONSchema7, HandlerMeta} from "bff-types";
export declare const meta: HandlerMeta[];
export declare const schema: JSONSchema7;
`;

const BUILD_DECODERS_DTS = `
import {JSONSchema7, HandlerMeta} from "bff-types";
type Decoders<T> = {
  [K in keyof T]: {
    parse: (input: any) => T[K];
  };
};
export declare const buildParsers: <T>() => Decoders<T>;
`;

const decodersCode = `
class CoercionFailure {}
function add_path_to_errors(errors, path) {
  return errors.map((e) => ({ ...e, path: [...path, ...e.path] }));
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
function coerce_union(input, ...cases) {
  for (const c of cases) {
    const r = coerce(c, input);
    if (!(r instanceof CoercionFailure)) {
      return r;
    }
  }
  return new CoercionFailure();
}
function coerce(coercer, value) {
  return coercer(value);
}
`;

const buildParsers = `
function buildParsers() {
  let decoders ={};
  Object.keys(buildParsersInput).forEach(k => {
    let v = buildParsersInput[k];
    decoders[k] = {
      parse: (input) => {
        const validation_result = v(input);
        if (validation_result.length === 0) {
          return input;
        }
        // TODO: throw a pretty error message
        throw validation_result
      }
    }
  });
  return decoders;
}
`;
const finalizeFile = (wasmCode: WritableModules, mod: ProjectModule) => {
  const exportCode = mod === "esm" ? "export " : "module.exports = ";
  const expr = [
    "meta",
    "schema",
    wasmCode.had_build_decoders_call ? "buildParsers" : "",
  ]
    .filter((it) => it.length > 0)
    .join(", ");
  const exports = [exportCode, `{ ${expr} };`].join(" ");
  const schema = ["const schema = ", wasmCode.json_schema, ";"].join(" ");
  return [
    decodersCode,
    wasmCode.js_server_data,
    wasmCode.had_build_decoders_call ? buildParsers : "",
    schema,
    exports,
  ].join("\n");
};

export const execProject = (
  projectPath: string,
  projectJson: ProjectJson,
  verbose: boolean
) => {
  const mod = projectJson.module ?? "esm";
  const bundler = new Bundler(verbose);
  const entryPoint = path.join(path.dirname(projectPath), projectJson.router);
  const outResult = bundler.bundle(entryPoint);
  if (outResult == null) {
    process.exit(1);
  }
  const outputDir = path.join(path.dirname(projectPath), projectJson.outputDir);
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir);
  }

  const outputFile = path.join(outputDir, "index.js");
  const finalFile = finalizeFile(outResult, mod);
  fs.writeFileSync(outputFile, finalFile);

  const outputDts = path.join(outputDir, "index.d.ts");
  fs.writeFileSync(
    outputDts,
    [
      RUNTIME_DTS,
      outResult.had_build_decoders_call ? BUILD_DECODERS_DTS : "",
    ].join("\n")
  );
  const outputSchemaJson = path.join(outputDir, "schema.json");
  fs.writeFileSync(outputSchemaJson, outResult.json_schema);
};
