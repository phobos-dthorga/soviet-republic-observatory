export const releaseGatePhases = Object.freeze([
  Object.freeze({
    id: "fast-contracts",
    label: "Fast contracts",
    command: "npm",
    args: Object.freeze(["run", "verify:fast"]),
  }),
  Object.freeze({
    id: "rust-tests",
    label: "Rust tests",
    command: "npm",
    args: Object.freeze(["run", "rust:test"]),
  }),
  Object.freeze({
    id: "rust-clippy",
    label: "Rust Clippy",
    command: "npm",
    args: Object.freeze(["run", "rust:clippy"]),
  }),
  Object.freeze({
    id: "browser-interface",
    label: "Browser interface audit",
    command: "npm",
    args: Object.freeze(["run", "verify:browser"]),
  }),
  Object.freeze({
    id: "desktop-package",
    label: "Desktop package",
    command: "npm",
    args: Object.freeze(["run", "desktop:build:binary"]),
    reuseAuditedWeb: true,
  }),
  Object.freeze({
    id: "native-smoke",
    label: "Native smoke review",
    command: "npm",
    args: Object.freeze(["run", "desktop:smoke:existing"]),
  }),
]);
