import wasm from "../pkg/hello_wasm";
import fs from "fs";
import { resolveModuleName } from "typescript";

const resolveImportNoCache = (
  file_name: string,
  mod: string
): string | undefined => {
  // if (isRelativeImport(mod)) {
  //   return resolveRelativeImport(file_name, mod);
  // }

  interface ModuleResolutionHost {
    fileExists(fileName: string): boolean;
    readFile(fileName: string): string | undefined;
    trace?(s: string): void;
    directoryExists?(directoryName: string): boolean;
    /**
     * Resolve a symbolic link.
     * @see https://nodejs.org/api/fs.html#fs_fs_realpathsync_path_options
     */
    realpath?(path: string): string;
    getCurrentDirectory?(): string;
    getDirectories?(path: string): string[];
    useCaseSensitiveFileNames?: boolean | (() => boolean) | undefined;
  }
  const host: ModuleResolutionHost = {
    fileExists: (file_name: string) => {
      return fs.existsSync(file_name);
    },
    readFile: function (fileName: string): string | undefined {
      return fs.readFileSync(fileName, "utf-8");
    },
  };
  const r = resolveModuleName(mod, file_name, {}, host);
  console.log(
    `JS: Resolved -import ? from '${mod}'- at ${file_name} => ${r.resolvedModule?.resolvedFileName}`
  );
  return r.resolvedModule?.resolvedFileName;

  // return undefined;
};
const resolvedCache: Record<string, Record<string, string | undefined>> = {};
const resolveImport = (file_name: string, mod: string): string | undefined => {
  // const cached = resolvedCache?.[file_name]?.[mod];
  // if (cached) {
  //   return cached;
  // }

  const result = resolveImportNoCache(file_name, mod);
  // if (result) {
  //   resolvedCache[file_name] = resolvedCache[file_name] || {};
  //   resolvedCache[file_name][mod] = result;
  // }
  return result;
};
globalThis.resolve_import = resolveImport;
globalThis.read_file_content = (file_name: string) => {
  try {
    const source_file = fs.readFileSync(file_name, "utf-8");
    return source_file;
  } catch (e) {
    return undefined;
  }
};
export class Bundler {
  seenFiles: Set<string> = new Set();
  constructor() {
    wasm.init();
  }

  public bundle(file_name: string): string {
    return wasm.bundle_to_string(file_name);
  }
}
