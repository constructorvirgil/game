import { spawn } from "node:child_process";
import { constants } from "node:fs";
import { access } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";

const repoRoot = path.resolve(process.cwd(), "..");
const exeName = process.platform === "win32" ? "server.exe" : "server";
const serverBin = path.join(repoRoot, "target", "debug", exeName);
const cargoBin = path.join(
  homedir(),
  ".cargo",
  "bin",
  process.platform === "win32" ? "cargo.exe" : "cargo",
);

async function exists(executablePath) {
  try {
    await access(executablePath, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

function run(cmd, args, cwd) {
  const child = spawn(cmd, args, {
    cwd,
    stdio: "inherit",
    shell: false,
  });

  const stop = () => {
    if (!child.killed) child.kill("SIGTERM");
  };
  process.on("SIGINT", stop);
  process.on("SIGTERM", stop);

  child.on("exit", (code) => {
    process.exit(code ?? 0);
  });
}

if (await exists(serverBin)) {
  run(serverBin, [], repoRoot);
} else if (await exists(cargoBin)) {
  run(cargoBin, ["run", "-p", "server"], repoRoot);
} else {
  run("cargo", ["run", "-p", "server"], repoRoot);
}
