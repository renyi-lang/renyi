// The client of the Renyi language server (decision AN2): starts `renyi
// lsp` for the workspace and lets the language client library do the
// rest (diagnostics, hover, go to definition, the outline). The binary is
// `renyi` on the PATH unless the setting `renyi.path` says otherwise.
"use strict";

const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const command = vscode.workspace.getConfiguration("renyi").get("path") || "renyi";
  const serverOptions = { command, args: ["lsp"] };
  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "renyi" }],
  };
  client = new LanguageClient("renyi", "Renyi", serverOptions, clientOptions);
  client.start().catch((error) => {
    vscode.window.showWarningMessage(
      `Renyi: the language server did not start (${error.message}). ` +
        `Is \`${command}\` installed and on the PATH? Set renyi.path otherwise.`
    );
  });
  context.subscriptions.push({ dispose: () => deactivate() });
}

function deactivate() {
  if (!client) {
    return undefined;
  }
  const stopping = client.stop();
  client = undefined;
  return stopping;
}

module.exports = { activate, deactivate };
