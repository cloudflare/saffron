import { execSync } from "child_process";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

// return value is the first directory created
const scripts_dir = path.dirname(fileURLToPath(import.meta.url));
const cfron_web_dir = path.join(scripts_dir, "..");
const pkg_dir = path.join(cfron_web_dir, "pkg");
const out_dir = path.join(cfron_web_dir, "out");
const wasm_pack_dir = path.join(out_dir, "wasm-pack");

execSync(
  `wasm-pack build --target bundler --out-dir ${wasm_pack_dir} --out-name cfron`,
  {
    cwd: cfron_web_dir,
    shell: true,
    stdio: "inherit",
  }
);

const bindgen_files = [
  "cfron_bg.d.ts",
  "cfron_bg.js",
  "cfron_bg.wasm",
].map((file) => [path.join(wasm_pack_dir, file), path.join(pkg_dir, file)]);

if (!fs.existsSync(pkg_dir)) {
  fs.mkdirSync(pkg_dir);
}

for (const [srcFile, destFile] of bindgen_files) {
  fs.copyFileSync(srcFile, destFile);
}
