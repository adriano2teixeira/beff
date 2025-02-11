"use strict";
import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import { Bundler, WasmDiagnosticInformation } from "./bundler";
import { BeffUserSettings, ProjectJson, parseUserSettings } from "./project";

const readProjectJson = (
  projectPath: string
): Pick<ProjectJson, "schema" | "module" | "parser" | "settings"> => {
  const projectJson = JSON.parse(fs.readFileSync(projectPath, "utf-8"));

  if (!projectJson.router && !projectJson.parser) {
    throw new Error(`Field "router" or "parser" not found in bff.json`);
  }
  return {
    schema: projectJson.schema == null ? projectJson.schema : String(projectJson.schema),
    parser: projectJson.parser == null ? projectJson.parser : String(projectJson.parser),
    module: projectJson.module,
    settings: parseUserSettings(projectJson),
  };
};

let crashed: any = false;

let bundler: Bundler | null = null;
const crashPopup = () => vscode.window.showErrorMessage(`BFF WASM CRASHED. Please reload VSCode. ${crashed}`);
const crashChecked = (cb: () => void) => {
  if (crashed) {
    crashPopup();
    return;
  }

  try {
    cb();
  } catch (e) {
    if (!crashed) {
      crashed = e;
    }
    crashPopup();
  }
};

const VERBOSE = false;

export function activate(_context: vscode.ExtensionContext) {
  const beffPath = String(vscode.workspace.getConfiguration("beff").get("configPath") ?? "beff.json");

  const workspacePath = vscode.workspace.workspaceFolders?.[0].uri.fsPath;
  if (!workspacePath) {
    throw new Error("No workspace folder found");
  }
  const collection = vscode.languages.createDiagnosticCollection("test");
  const projectPath = path.join(workspacePath, beffPath);
  const projectJson = readProjectJson(projectPath);

  bundler = new Bundler(VERBOSE);
  // const router = projectJson.router;
  const schema_entrypoint =
    projectJson.schema == null ? undefined : path.join(path.dirname(projectPath), projectJson.schema);
  const parser_entrypoint =
    projectJson.parser == null ? undefined : path.join(path.dirname(projectPath), projectJson.parser);

  const updateDiag = () =>
    updateDiagnostics(schema_entrypoint, parser_entrypoint, projectJson.settings, collection);
  updateDiag();
  const watcher = vscode.workspace.createFileSystemWatcher(
    new vscode.RelativePattern(workspacePath, "**/*.ts")
  );
  const observingExtensions = [".ts", ".tsx", ".d.ts", ".cts", ".mts"];

  watcher.onDidChange((e) => {
    crashChecked(() => {
      // eslint-disable-next-line no-console
      console.log("File changed1: ", e.fsPath);
      if (!observingExtensions.includes(path.extname(e.fsPath))) {
        return;
      }
      const newContent = fs.readFileSync(e.fsPath, "utf-8");
      bundler?.updateFileContent(e.fsPath, newContent);
      updateDiag();
    });
  });
  vscode.workspace.onDidChangeTextDocument((e) => {
    crashChecked(() => {
      // eslint-disable-next-line no-console
      console.log("File changed2: ", e.document.uri.fsPath);
      if (!observingExtensions.includes(path.extname(e.document.uri.fsPath))) {
        return;
      }
      const newContent = e.document.getText();
      bundler?.updateFileContent(e.document.uri.fsPath, newContent);
      updateDiag();
    });
  });
}

const getFileNameFromDiag = (diag: WasmDiagnosticInformation) => {
  if (diag.KnownFile) {
    return diag.KnownFile.file_name;
  }
  return diag.UnknownFile.current_file;
};

const relatedInformation = (cause: WasmDiagnosticInformation): vscode.DiagnosticRelatedInformation => {
  if (cause.KnownFile) {
    const diag = cause.KnownFile;
    return {
      message: diag.message,
      location: new vscode.Location(
        vscode.Uri.file(diag.file_name),
        new vscode.Range(
          new vscode.Position(diag.line_lo - 1, diag.col_lo),
          new vscode.Position(diag.line_hi - 1, diag.col_hi)
        )
      ),
    };
  } else {
    const diag = cause.UnknownFile;
    return {
      message: diag.message,
      location: new vscode.Location(
        vscode.Uri.file(diag.current_file),
        new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0))
      ),
    };
  }
};

function updateDiagnostics(
  schema_entrypoint: string | undefined,
  parser_entrypoint: string | undefined,
  settings: BeffUserSettings,
  collection: vscode.DiagnosticCollection
): void {
  collection.clear();
  const diags = bundler?.diagnostics(parser_entrypoint, schema_entrypoint, settings);
  const acc: Record<string, vscode.Diagnostic[]> = {};
  const pushDiag = (k: string, v: vscode.Diagnostic) => {
    if (acc[k] == null) {
      acc[k] = [];
    }
    acc[k].push({ ...v, source: "beff" });
  };
  (diags?.diagnostics ?? []).forEach((data) => {
    const cause = data.cause;
    if (cause.KnownFile) {
      const diag = cause.KnownFile;
      pushDiag(getFileNameFromDiag(cause), {
        message: (data.message ? data.message + " - " : "") + diag.message,
        range: new vscode.Range(
          new vscode.Position(diag.line_lo - 1, diag.col_lo),
          new vscode.Position(diag.line_hi - 1, diag.col_hi)
        ),
        severity: vscode.DiagnosticSeverity.Error,
        relatedInformation: (data.related_information ?? []).map(relatedInformation),
      });
    } else {
      const diag = cause.UnknownFile;
      pushDiag(diag.current_file, {
        message: (data.message ? data.message + " - " : "") + diag.message,
        range: new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0)),
        severity: vscode.DiagnosticSeverity.Error,
      });
    }
    data.related_information?.forEach((related) => {
      if (related.KnownFile) {
        const diag = related.KnownFile;
        pushDiag(getFileNameFromDiag(related), {
          message: diag.message,
          range: new vscode.Range(
            new vscode.Position(diag.line_lo - 1, diag.col_lo),
            new vscode.Position(diag.line_hi - 1, diag.col_hi)
          ),
          severity: vscode.DiagnosticSeverity.Warning,
        });
      } else {
        const diag = related.UnknownFile;
        pushDiag(diag.current_file, {
          message: diag.message,
          range: new vscode.Range(new vscode.Position(0, 0), new vscode.Position(0, 0)),
          severity: vscode.DiagnosticSeverity.Warning,
        });
      }
    });
  });
  Object.keys(acc).forEach((k) => {
    const documentUri = vscode.Uri.file(k);
    collection.set(documentUri, acc[k]);
  });
}
