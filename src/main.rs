//! Brief's sandboxed Python guest, compiled to wasm32-wasip1 and run under
//! wasmtime with NO network and NO preopened directories. The WASI runtime
//! is the security boundary: with host_env off there is no `os`, no
//! `socket`, no `_ctypes`, no file `open()` — and WASI grants nothing back —
//! so escape attempts fail by construction, not by denylist.
//!
//! Protocol: read one JSON envelope {script, documents} on stdin, run the
//! script with a pure-Python `brief` module (reads served from the passed
//! documents; artefact writes collected as data), emit one JSON envelope
//! {output, artifacts, error} on stdout. The host renders the collected
//! artefacts with its own Rust docx/xlsx renderers — the guest never touches
//! the filesystem.

use rustpython_vm as vm;
use std::io::Read;

// host_env is off, so RustPython can't wire its own stdout; capture is a
// pure-Python shim (no `io` module, whose frozen io.py needs the absent
// _io.FileIO). exec() runs the user script; traceback formats errors;
// json (de)serialises the envelopes.
const BOOTSTRAP: &str = r#"
import json, sys

_env = json.loads(_stdin)
_docs = _env.get("documents", {})
_script = _env.get("script", "")

class _Cap:
    def __init__(self): self.b = []
    def write(self, s): self.b.append(str(s)); return len(s)
    def flush(self): pass
_cap = _Cap()
sys.stdout = sys.stderr = _cap

class _Brief:
    def __init__(self, docs): self._docs = docs; self.artifacts = []
    def list_documents(self): return list(self._docs.keys())
    def read_document(self, name):
        if name not in self._docs: raise ValueError("no document named %r" % name)
        return "\n".join("--- page %d ---\n%s" % (i + 1, p) for i, p in enumerate(self._docs[name]))
    def read_page(self, name, page):
        if name not in self._docs: raise ValueError("no document named %r" % name)
        pg = self._docs[name]
        if not (1 <= page <= len(pg)): raise ValueError("%s has no page %d" % (name, page))
        return pg[page - 1]
    def save_docx(self, filename, markdown):
        self.artifacts.append({"name": filename, "kind": "docx", "content": str(markdown)}); return filename
    def save_xlsx(self, filename, rows):
        self.artifacts.append({"name": filename, "kind": "xlsx", "rows": rows}); return filename
    def save_text(self, filename, content):
        self.artifacts.append({"name": filename, "kind": "text", "content": str(content)}); return filename

brief = _Brief(_docs)
sys.modules["brief"] = brief

_err = None
_g = {"__name__": "__main__", "brief": brief}
try:
    exec(compile(_script, "<script>", "exec"), _g)
except BaseException as _e:
    # traceback/io are unavailable (host_env off); format by hand.
    _err = "%s: %s" % (type(_e).__name__, _e)

__result = json.dumps({"output": "".join(_cap.b), "artifacts": brief.artifacts, "error": _err})
"#;

fn main() {
    let mut stdin = String::new();
    if std::io::stdin().read_to_string(&mut stdin).is_err() {
        print!("{{\"output\":\"\",\"artifacts\":[],\"error\":\"could not read input\"}}");
        return;
    }

    let builder = vm::Interpreter::builder(vm::Settings::default());
    let stdlib = rustpython_stdlib::stdlib_module_defs(&builder.ctx);
    let interp = builder
        .add_native_modules(&stdlib)
        .add_frozen_modules(rustpython_pylib::FROZEN_STDLIB)
        .build();

    let result = interp.enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        let stdin_obj = vm.ctx.new_str(stdin.as_str());
        if scope.globals.set_item("_stdin", stdin_obj.into(), vm).is_err() {
            return None;
        }
        if let Err(e) = vm.run_string(scope.clone(), BOOTSTRAP, "<bootstrap>".to_owned()) {
            // A failure in the harness itself (not the user script, which is
            // caught in Python) — surface it as a structured error.
            let mut s = String::new();
            let _ = vm.write_exception(&mut s, &e);
            return Some(format!(
                "{{\"output\":\"\",\"artifacts\":[],\"error\":{}}}",
                json_string(&s)
            ));
        }
        scope
            .globals
            .get_item("__result", vm)
            .ok()
            .and_then(|v| v.str(vm).ok())
            .map(|s| s.to_string_lossy().into_owned())
    });

    match result {
        Some(json) => print!("{json}"),
        None => print!("{{\"output\":\"\",\"artifacts\":[],\"error\":\"interpreter harness failed\"}}"),
    }
}

/// Minimal JSON string escaping for the fallback error path.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
