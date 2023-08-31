"use strict";
import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import { Bundler } from "./bundler";
import { ProjectJson } from "./project";

const readProjectJson = (projectPath: string): ProjectJson => {
  const projectJson = JSON.parse(fs.readFileSync(projectPath, "utf-8"));
  if (!projectJson.router) {
    throw new Error("router not found in project.json");
  }
  if (!projectJson.outputDir) {
    throw new Error("outputDir not found in project.json");
  }

  return {
    router: String(projectJson.router),
    outputDir: String(projectJson.outputDir),
  };
};

let bundler: Bundler | null = null;
export function activate(context: vscode.ExtensionContext) {
  bundler = new Bundler(false);

  const workspacePath = vscode.workspace.workspaceFolders?.[0].uri.fsPath;
  if (!workspacePath) {
    throw new Error("No workspace folder found");
  }
  const collection = vscode.languages.createDiagnosticCollection("test");

  const projectPath = path.join(workspacePath, "bff.json");
  const projectJson = readProjectJson(projectPath);
  const router = projectJson.router;
  const entryPoint = path.join(path.dirname(projectPath), router);

  updateDiagnostics(entryPoint, collection);

  const watcher = vscode.workspace.createFileSystemWatcher(
    new vscode.RelativePattern(workspacePath, "**/*.ts")
  );

  watcher.onDidChange((e) => {
    console.log("File changed: ", e.fsPath);
    const newContent = fs.readFileSync(e.fsPath, "utf-8");
    bundler?.updateFileContent(e.fsPath, newContent);
    updateDiagnostics(entryPoint, collection);
  });

  vscode.workspace.onDidChangeTextDocument((e) => {
    console.log("File changed: ", e.document.uri.fsPath);
    const newContent = e.document.getText();
    bundler?.updateFileContent(e.document.uri.fsPath, newContent);
    updateDiagnostics(entryPoint, collection);
  });
}

function updateDiagnostics(
  entryPoint: string,
  collection: vscode.DiagnosticCollection
): void {
  collection.clear();
  const diags = bundler?.diagnostics(entryPoint);
  (diags?.diagnostics ?? []).forEach((diag) => {
    const documentUri = vscode.Uri.file(diag.file_name);
    collection.set(documentUri, [
      {
        code: "",
        message: diag.message,
        range: new vscode.Range(
          new vscode.Position(diag.line_lo - 1, diag.col_lo),
          new vscode.Position(diag.line_hi - 1, diag.col_hi)
        ),
        severity: vscode.DiagnosticSeverity.Error,
        source: "",
        relatedInformation: [
          //   new vscode.DiagnosticRelatedInformation(
          //     new vscode.Location(
          //         documentUri,
          //       new vscode.Range(
          //         new vscode.Position(1, 8),
          //         new vscode.Position(1, 9)
          //       )
          //     ),
          //     "first assignment to `x`"
          //   ),
        ],
      },
    ]);
  });
}
